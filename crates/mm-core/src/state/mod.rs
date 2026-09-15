// (C) 2025-2026 MWBM Partners Ltd
//
// The write lock: making sure only one batch of file moves runs at a time.
//
// What this is for
// ----------------
// Renaming a library is not a single action — it is a plan worked out up
// front (this file goes there, that file goes here) and then carried out one
// file at a time. If a second copy of MeedyaManager starts carrying out its
// own plan halfway through, the two plans disagree about where things are:
// one copy moves a file the other copy is still expecting to find, and both
// finish reporting success while a file has quietly gone somewhere nobody
// asked for.
//
// So this lock is deliberately narrow. It is held **only while a batch of
// renames is actually being carried out**, and released the moment that
// batch finishes. It is *not* a rule that you may only run one copy of
// MeedyaManager: reading tags with `meedya debug`, previewing a rename, or
// leaving a folder watcher running are all perfectly safe alongside anything
// else, and none of them takes this lock.
//
// How it works now, and why the old design was replaced
// -------------------------------------------------------
// The previous version of this module kept its own idea of a lock: a file
// holding a process number, created with `create_new` so two processes could
// not both create it at once, and cleared away by hand if the process it
// named had died. That was reviewed and found to be unsafe in three separate
// ways (issue #49, review round):
//
//   * clearing a leftover lock was four separate steps — read the file, ask
//     the operating system whether that process is alive, delete the file,
//     create a new one — and two processes could each get through the first
//     two steps before either reached the third, so both could end up
//     believing they held an exclusive lock at once (#6);
//   * if the process number written in the file was ever reused by some
//     unrelated program later — which operating systems do, process numbers
//     are recycled — every future rename was refused forever, with a message
//     telling the user to go and stop a program that had nothing to do with
//     MeedyaManager (#7); and
//   * a write that failed part-way through left an empty file behind, which
//     could not be parsed as a process number and was therefore treated as a
//     lock held by nobody knowable — refusing every future run (#16).
//
// The replacement asks the operating system to do the one thing operating
// systems are actually good at here: `std::fs::File::try_lock` puts an
// exclusive lock on the file itself. Two processes (or two handles opened by
// the *same* process — see the tests) racing to lock the same file cannot
// both win, with no read-then-write gap for either of the old bugs to live
// in. And critically, **the operating system itself gives the lock up the
// moment the process holding it ends** — however it ends: an orderly return,
// an early return, a panic that unwinds, `panic = "abort"` (the release
// profile in this workspace's `Cargo.toml`, under which no destructor ever
// runs), or `kill -9`. There is no PID to go stale, because nothing here
// checks a PID to decide whether the lock is free.
//
// What this does and does not cover
// ----------------------------------
// * It only ever covers copies of MeedyaManager using the *same settings
//   folder* on the *same computer*. Two different settings folders, or two
//   different computers each writing to a shared network folder without
//   locking working over the network (see below), are not covered by this at
//   all — and even a single computer writing to a network folder is only
//   covered as far as that network filesystem's own locking actually behaves
//   like a local disk's, which is not guaranteed (see the NFS caveat below).
// * On macOS and Linux the lock is *advisory*, using `flock` under the hood.
//   That only stops other programs that also ask the operating system for
//   the same lock before touching the file — which is fine, because the only
//   program that ever asks for this particular lock is MeedyaManager itself.
//   It is not a permissions barrier: **deleting `meedya.lock` by hand while
//   it is held defeats the whole mechanism** on these two platforms, because
//   there is nothing stopping the delete. Don't do that; there is never a
//   good reason to.
// * On Windows the lock is *mandatory* (`LockFileEx`), but not in quite the
//   way an earlier version of this comment claimed. **Correction (issue
//   #49, review round):** a `LockFileEx` lock does not stop a second handle
//   from *opening* the file at all — that open (see `open_options.open`
//   below) succeeds perfectly normally for a second copy of MeedyaManager.
//   What the lock stops is that second handle *reading or writing the
//   locked bytes*: its own `try_lock()` call is what then reports "would
//   block". What separately stops the file being deleted or replaced out
//   from under us is the **share mode** passed to `open_options` — a
//   different Windows mechanism from the lock itself, granting
//   `FILE_SHARE_READ`/`FILE_SHARE_WRITE` but deliberately withholding
//   `FILE_SHARE_DELETE` (see the comment at the share-mode call site below).
//   That distinction — the open succeeds, the lock is what refuses — is
//   exactly why the process number and the reason it was taken have to live
//   in a *separate* file (`meedya.lock.info`, see [`LockFile::holder`])
//   rather than inside `meedya.lock` itself: a second copy's own open of the
//   lock file succeeds, but it can never read the bytes inside it while we
//   hold the lock.
// * On a network drive, taking a file lock may not be supported at all by
//   the underlying filesystem. When that happens `try_acquire` returns an
//   error rather than silently proceeding unlocked — see its documentation,
//   and `lock_unavailable_message` below for how that is put to the user.
//   **Caveat added in review:** "not supported" is the simple case. On
//   Linux, a mount over NFS more often does not refuse the lock outright —
//   the kernel emulates `flock` using NFS's own byte-range locking, which
//   follows different rules: two handles opened by the *same* process can
//   fail to exclude each other (unlike the same case on a local disk, which
//   this module's own tests rely on), and closing *any one* of a process's
//   handles to the file can drop the lock for all of them. So on NFS the
//   lock is often "supported" in the sense of never erroring, but with
//   weaker guarantees than the rest of this module assumes, rather than
//   being refused outright the way a genuinely unsupported filesystem is.
//
// Never delete `meedya.lock`
// ---------------------------
// The lock file itself is created once and then left alone forever. It is
// never deleted by anything in this module, on purpose: what makes the old
// design's cleanup dance (#6 above) possible at all is that a lock could be
// taken to mean "the file exists", so *removing the file* was how you gave
// the lock up — and that step could race with somebody else about to create
// a fresh one. If nothing ever deletes the file, that whole class of race
// cannot happen. Giving the lock up is done by unlocking the file handle
// (which closing it, including at process exit, does on its own) — never by
// touching the file's existence on disk.

use std::path::{Path, PathBuf};

use chrono::Utc;
use tracing::{debug, info, warn};

/// The write lock held while a batch of renames is being carried out.
///
/// Create one with [`LockFile::try_acquire_default`] just before moving any
/// files, and keep the value alive for as long as the moving goes on. Giving
/// it up — by calling [`LockFile::release`], or simply letting the value go
/// out of scope — never deletes `meedya.lock`; see the module docs for why
/// that is deliberate.
///
/// See the notes at the top of this module for what the lock does and does
/// not cover — in particular, it does **not** stop a second copy of
/// MeedyaManager from running; it only stops a second copy from *moving
/// files* at the same time.
pub struct LockFile {
    /// The open handle the operating system's lock is attached to. Holding
    /// this — not any particular content inside the file — is what the lock
    /// *is*. Dropping it (including the operating system doing so on our
    /// behalf when the process ends) is what releases it.
    file: std::fs::File,
    /// Where `meedya.lock` lives, kept so `holder`/`release` can find the
    /// `.info` note that sits beside it.
    path: PathBuf,
}

/// Best-effort information about whoever holds (or last held) a write lock,
/// read from the note beside it — see [`LockFile::holder`].
///
/// **This is for messages only.** Nothing in this module uses it to decide
/// whether a lock is free; that decision belongs to the operating system
/// alone, inside [`LockFile::try_acquire`]. Every field is independently
/// optional because the note is written on a best-effort basis (see
/// `write_info`) and parsed leniently — a line this build does not recognise,
/// or a value that fails to parse, is simply left out rather than failing
/// the whole read.
#[derive(Debug, Clone)]
pub struct LockHolder {
    /// The process number that took the lock, if the note named one and it
    /// parsed as a number.
    pub pid: Option<u32>,
    /// When the lock was taken, as the raw RFC 3339 text that was written —
    /// callers that want to show it nicely (see `scan.rs`'s blocked message)
    /// parse it themselves rather than this module deciding a display format
    /// on their behalf.
    pub started: Option<String>,
    /// The short, human-readable description of what was about to happen,
    /// supplied by the caller that took the lock (see `try_acquire`).
    pub purpose: Option<String>,
}

/// Describe whoever holds the write lock in a form a person can act on —
/// `"process 4321 (`meedya scan --execute`, started 10:42)"` — or `None` if
/// nothing readable was recorded (see [`LockFile::holder`], which is
/// best-effort by design: a full disk when the note was written, or simply
/// nobody having taken the lock since this settings folder was last used,
/// both look the same from here).
///
/// The stored timestamp is RFC 3339 text in UTC; it is parsed back out and
/// shown in the local clock's hours and minutes, because "10:42" means
/// something to somebody reading a terminal and an ISO instant does not.
fn describe_holder(holder: &LockHolder) -> Option<String> {
    let pid = holder.pid?;
    let purpose = holder.purpose.as_deref().unwrap_or("an unknown command");
    let when = holder
        .started
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_else(|| "an unknown time".to_string());
    Some(format!("process {pid} (`{purpose}`, started {when})"))
}

/// Build the message a blocked caller shows when another copy of
/// MeedyaManager already holds the write lock — the ordinary, expected
/// `Ok(None)` outcome from [`LockFile::try_acquire`], never a failure.
///
/// **Why this lives here rather than in the command line or the FFI layer
/// (issue #49, review round).** Before this moved, `crates/mm-cli`'s `scan`
/// command built this exact wording locally, while `crates/mm-ffi`'s
/// `execute_renames` built its own separate text that named no holder at
/// all and said "or stop it" — and stopping a copy of MeedyaManager
/// part-way through a batch of renames can leave a library half-moved,
/// which is exactly why that advice must never appear. Sharing one function
/// is what stops the command line and the desktop apps disagreeing about
/// what a blocked user is told.
///
/// `holder` comes from [`LockFile::holder`], read against the default lock
/// path at the moment the caller was refused — see this module's private
/// `describe_holder` helper for why it can be `None` even while the lock
/// genuinely is held by somebody.
pub fn lock_busy_message(holder: Option<&LockHolder>) -> String {
    // Built as a single `format!` rather than several `push_str`/`format!`
    // appends — clippy's `format_push_string` lint flags the latter because
    // it allocates once per append; one format call is both cheaper and, for
    // a three-sentence message like this, no harder to read.
    let holder_sentence = holder
        .and_then(describe_holder)
        .map(|description| format!(" It appears to be {description}."))
        .unwrap_or_default();

    format!(
        "Another copy of MeedyaManager is moving files right now, so nothing has been \
         moved.{holder_sentence} Wait for it to finish, then run this again."
    )
}

/// Build the message shown when the write lock could not even be attempted.
///
/// As opposed to [`lock_busy_message`], which is for the ordinary "somebody
/// else already has it" case. This is the rarer, more serious case:
/// something about the settings folder itself refused, and
/// [`LockFile::try_acquire`] always refuses to move anything rather than
/// guess, because guessing and proceeding unlocked is exactly the
/// two-processes-moving-files-at-once bug this lock exists to prevent.
///
/// **Why the wording changed (issue #49, review round).** The previous
/// version of this message told the reader to set `MM_CONFIG_DIR` to a
/// different folder. Every part of that advice was wrong:
///
/// * it separates this run from every other copy of MeedyaManager — the
///   desktop app and the background service go on using the *original*
///   settings folder, so a terminal run pointed elsewhere stops excluding
///   them, which defeats the entire point of the lock;
/// * the new folder has no `settings.json5` in it, so every configured
///   setting silently reverts to its default the moment somebody follows
///   the advice;
/// * the new folder has no Test Mode manifest either, so
///   `test_mode::load_manifest` returns its defaults there too — a Test
///   Mode user's real files would then be moved as though Test Mode were
///   off, which is precisely the harm Test Mode exists to prevent; and
/// * it blamed network drives even for a plain "Permission denied", which
///   has nothing to do with what kind of drive is involved.
///
/// So this never suggests moving anything, deleting anything, or working
/// around the problem — it only ever refuses, plainly, with the real
/// reason attached. Takes the underlying `std::io::Error` directly (rather
/// than an already-formatted `MmError`) specifically so the original
/// `std::io::ErrorKind` survives long enough to decide whether the
/// network-drive caveat below is actually warranted — an `MmError::State`
/// only ever carries pre-formatted text, which has already lost that.
pub fn lock_unavailable_message(err: &std::io::Error) -> String {
    // A network drive is mentioned only when the operating system itself
    // says locking is not supported there at all (`ErrorKind::Unsupported`)
    // — never merely because the failing call happened to be a filesystem
    // one. An ordinary permissions problem is not evidence of a network
    // drive, and blaming one for it was exactly the mistake found in
    // review.
    let network_caveat = if err.kind() == std::io::ErrorKind::Unsupported {
        " This can happen when the drive holding MeedyaManager's settings does not support \
         file locking (this happens on some network drives)."
    } else {
        ""
    };

    format!(
        "MeedyaManager could not take its write lock, so nothing has been moved ({err}).\
         {network_caveat}"
    )
}

impl LockFile {
    /// Try to take the write lock at `path`.
    ///
    /// `purpose` is a short, human-readable description of what is about to
    /// happen — `"meedya scan --execute"`, say. It is written into the note
    /// beside the lock purely so a blocked caller can be told something
    /// useful later (see [`LockFile::holder`]); it plays no part in deciding
    /// whether the lock is free.
    ///
    /// Returns:
    /// * `Ok(Some(lock))` — nobody else held it; it is now ours.
    /// * `Ok(None)` — somebody else holds it right now. This is the ordinary,
    ///   expected "busy" outcome and must not be treated as a failure by
    ///   callers — see [`lock_busy_message`] for how to report it.
    /// * `Err(_)` — the lock could not even be attempted: the containing
    ///   folder could not be created, the file could not be opened, or
    ///   locking is not supported here at all (some network filesystems do
    ///   not support file locking). Callers must refuse to move anything
    ///   rather than guess — proceeding unlocked is exactly the
    ///   two-processes-moving-files-at-once bug this module exists to close.
    ///   Returned as a plain `std::io::Error` (rather than folded into
    ///   `MmError` here) precisely so the original `std::io::ErrorKind`
    ///   reaches [`lock_unavailable_message`] intact — that is what decides
    ///   whether the network-drive caveat belongs in what the user is told.
    #[allow(clippy::incompatible_msrv)]
    // `std::fs::File::try_lock` was stabilised in Rust 1.89. The toolchain
    // pinned in `rust-toolchain.toml` is 1.98 and has been confirmed to have
    // it (see the task's scratch verification), but clippy checks method
    // availability against the workspace's *declared* `rust-version =
    // "1.85"` in the root `Cargo.toml`, so it flags this call as newer than
    // the minimum we advertise. Raising that declared minimum is a decision
    // for its own commit and issue — not something to slip in as a side
    // effect of this one.
    pub fn try_acquire(path: &Path, purpose: &str) -> std::io::Result<Option<Self>> {
        // The lock lives in the configuration directory, which may not exist
        // yet on a first run.
        if let Some(parent) = path.parent() {
            // Re-wrapped with `io::Error::new` rather than passed straight
            // through: that keeps the original `ErrorKind` intact (needed by
            // `lock_unavailable_message`'s network-drive check) while still
            // recording *which* of the three things that can fail here —
            // creating the folder, opening the file, or taking the lock —
            // actually happened, which the bare underlying error on its own
            // would not say.
            std::fs::create_dir_all(parent).map_err(|e| {
                std::io::Error::new(
                    e.kind(),
                    format!(
                        "cannot create the settings folder {}: {e}",
                        parent.display()
                    ),
                )
            })?;
        }

        // `create(true)` *without* `truncate` — deliberately. Emptying the
        // file on every open would be harmless for what we actually use it
        // for (nothing reads its content), but there is no reason to do it,
        // and the brief for this stage is explicit that the file must never
        // be emptied on open, only ever created if missing.
        let mut open_options = std::fs::OpenOptions::new();
        open_options.read(true).write(true).create(true);

        // Windows-only: deny other handles the ability to delete, rename or
        // replace this file while we hold it — a *separate* mechanism from
        // the lock taken below; see the corrected note in the module docs
        // above on what each of the two actually does. `FILE_SHARE_READ`
        // (0x1) and `FILE_SHARE_WRITE` (0x2) are granted; `FILE_SHARE_DELETE`
        // (0x4) is deliberately left out. Written as the plain numeric
        // constants (rather than pulling in `winapi`/`windows-sys` for two
        // flags) with this comment recording what they are.
        //
        // **Correction (issue #49, review round):** an earlier version of
        // this comment said `mm-core`'s Windows dependency on `libc`/
        // `winapi` was "only ever for the PID-checking code this stage
        // deletes" — worded in a way that implied this stage had removed
        // that dependency. It has not. `libc` and `winapi` are simply no
        // longer *used* by this module, which is a different thing from
        // being removed: both are still declared as dependencies in
        // `crates/mm-core/Cargo.toml`, and taking them out is left to a
        // separate manifest change rather than being folded in here.
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            const FILE_SHARE_READ: u32 = 0x1;
            const FILE_SHARE_WRITE: u32 = 0x2;
            open_options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
        }

        let file = open_options.open(path).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("cannot open the lock file {}: {e}", path.display()),
            )
        })?;

        match file.try_lock() {
            Ok(()) => {
                let lock = Self {
                    file,
                    path: path.to_path_buf(),
                };
                // Best effort — see `write_info`. Never lets a note-writing
                // failure decide whether we hold the actual lock, which we
                // already do at this point.
                lock.write_info(purpose);
                info!(
                    "write lock acquired: {} (PID {}, {purpose})",
                    path.display(),
                    std::process::id()
                );
                Ok(Some(lock))
            }
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(std::fs::TryLockError::Error(e)) => Err(std::io::Error::new(
                e.kind(),
                format!("cannot lock {}: {e}", path.display()),
            )),
        }
    }

    /// Try to take the write lock at the standard place for this platform.
    ///
    /// This is what callers should use: it keeps every part of the
    /// application — the command line, the desktop apps, the folder watcher
    /// — agreeing on *which* file is the lock, which is the whole point.
    pub fn try_acquire_default(purpose: &str) -> std::io::Result<Option<Self>> {
        Self::try_acquire(&Self::default_path(), purpose)
    }

    /// Read the note left beside `path` about whoever holds — or last held —
    /// the lock there. **For messages only** — see the type-level docs on
    /// [`LockHolder`]; nothing here decides whether the lock is actually
    /// free.
    ///
    /// Returns `None` if the note cannot be read at all (it was never
    /// written, a previous holder's write failed, or the lock has since been
    /// released and its note cleared — see `release`) or carries none of the
    /// three fields this reads.
    pub fn holder(path: &Path) -> Option<LockHolder> {
        let contents = std::fs::read_to_string(Self::info_path_for(path)).ok()?;

        let mut pid = None;
        let mut started = None;
        let mut purpose = None;

        // Plain `key=value` lines, not JSON — this file is written and read
        // entirely within this one module, so there is no format to keep
        // compatible with anything else, and a reader stumbling on it by
        // hand (or a future version reading an older one, or vice versa)
        // should be able to make sense of it without a schema. Unknown keys
        // and unparsed values are simply skipped rather than failing the
        // whole read — a note is either informative or absent, never wrong.
        for line in contents.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "pid" => pid = value.trim().parse::<u32>().ok(),
                "started" => started = Some(value.trim().to_string()),
                "purpose" => purpose = Some(value.trim().to_string()),
                _ => {}
            }
        }

        if pid.is_none() && started.is_none() && purpose.is_none() {
            // Nothing recognisable — most commonly an empty file, which is
            // exactly what `release` leaves behind on purpose.
            return None;
        }

        Some(LockHolder {
            pid,
            started,
            purpose,
        })
    }

    /// Give the lock up. Calling this is optional — dropping the value does
    /// the same thing — it exists for a caller that wants to give the lock
    /// up well before it would naturally go out of scope, where writing
    /// `drop(lock)` would read oddly next to code that never otherwise
    /// mentions Rust's `Drop` trait by name.
    ///
    /// `meedya.lock` itself is never deleted — see the module docs for why.
    /// What *does* happen, in order, while the lock is still held throughout:
    /// the `.info` note beside it is emptied, so a caller reading it via
    /// [`LockFile::holder`] a moment later sees "nobody" rather than a stale
    /// process number that no longer means anything; then the file is
    /// unlocked.
    pub fn release(self) {
        // Consuming `self` and doing nothing else with it here is
        // deliberate: it lets `Drop::drop` below do the actual work — while
        // every field is still intact — exactly once, from a single place,
        // whether a caller called this explicitly or simply let the value go
        // out of scope.
        drop(self);
    }

    /// Where the lock file lives on this platform.
    pub fn default_path() -> PathBuf {
        // Route through the single config-dir resolver (honours
        // `MM_CONFIG_DIR`) rather than calling `dirs::config_dir()` directly —
        // see issue #212 (P0-CONFIGDIR). Preserve the existing "." fallback
        // behaviour on error.
        let config_dir = crate::config::app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
        config_dir.join("meedya.lock")
    }

    /// Path to the note living beside `lock_path` — `meedya.lock.info` next
    /// to `meedya.lock`.
    ///
    /// Built by appending to the whole file name rather than via
    /// `Path::with_extension`, which would replace `.lock` outright and give
    /// `meedya.info` — losing the part of the name that says what it is a
    /// note *about*.
    fn info_path_for(lock_path: &Path) -> PathBuf {
        let mut name = lock_path.file_name().unwrap_or_default().to_os_string();
        name.push(".info");
        lock_path.with_file_name(name)
    }

    /// Write who holds the lock, when, and why, to the `.info` note beside
    /// it — best effort.
    ///
    /// "Best effort" is not a shrug: it is the deliberate choice that a
    /// failure to write a *message-only* note (a full disk, say) must never
    /// change whether the caller believes it holds the *actual* lock, which
    /// by the time this runs, it already genuinely does.
    fn write_info(&self, purpose: &str) {
        let info_path = Self::info_path_for(&self.path);
        let contents = format!(
            "pid={}\nstarted={}\npurpose={purpose}\n",
            std::process::id(),
            Utc::now().to_rfc3339(),
        );
        if let Err(e) = std::fs::write(&info_path, contents) {
            warn!(
                "could not write write-lock info note {}: {e}",
                info_path.display()
            );
        }
    }
}

impl Drop for LockFile {
    #[allow(clippy::incompatible_msrv)]
    // `std::fs::File::unlock` was stabilised alongside `try_lock` in Rust
    // 1.89 — see the identical allowance and comment on `try_acquire` above;
    // the reasoning is the same call for call.
    fn drop(&mut self) {
        // Emptied, not deleted — see the module docs on why `meedya.lock`
        // itself is never removed. Best effort, same reasoning as
        // `write_info`: a reader seeing a stale note for a little longer
        // than ideal is a cosmetic problem; the unlock below is the part
        // that actually matters and must happen regardless.
        let info_path = Self::info_path_for(&self.path);
        if let Err(e) = std::fs::write(&info_path, "") {
            warn!(
                "could not clear write-lock info note {}: {e}",
                info_path.display()
            );
        }

        // Not strictly necessary on its own — `self.file` is about to be
        // dropped a moment later regardless, and closing the last handle to
        // a locked file releases the lock on both the platforms this ships
        // on. Calling it explicitly first means the release happens at a
        // point this code controls, with something for `debug!` to log,
        // rather than merely "whenever field-drop order gets to it".
        if let Err(e) = self.file.unlock() {
            warn!("could not unlock {}: {e}", self.path.display());
        }
        debug!("write lock released: {}", self.path.display());
    }
}

#[cfg(test)]
#[allow(unsafe_code)] // Tests use set_var/remove_var which require unsafe in Edition 2024
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── The happy path ───────────────────────────────────────────────────

    #[test]
    fn try_acquire_succeeds_when_nothing_else_holds_the_lock() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        let lock = LockFile::try_acquire(&path, "test")
            .expect("acquiring must not error")
            .expect("nothing else holds this lock");
        assert!(path.is_file(), "the lock file must exist once taken");

        drop(lock);
    }

    /// A path whose parent directory does not exist yet (a first run, before
    /// the configuration directory has ever been created) must still work.
    #[test]
    fn try_acquire_creates_missing_parent_directories() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("deeper").join("test.lock");

        let lock = LockFile::try_acquire(&path, "test")
            .expect("acquiring must not error")
            .expect("nothing else holds this lock");
        assert!(path.is_file());

        drop(lock);
    }

    #[test]
    fn a_second_attempt_is_refused_while_the_first_still_holds_it() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        let _first = LockFile::try_acquire(&path, "first").unwrap();

        let second = LockFile::try_acquire(&path, "second")
            .expect("a busy lock is not an error — it is Ok(None)");
        assert!(
            second.is_none(),
            "a second attempt must be refused while the first still holds it"
        );
    }

    /// Two threads racing to take the same lock: exactly one may win.
    ///
    /// This property held under the old PID-file design too — `create_new`
    /// gave it the same one-indivisible-step guarantee this rewrite gets
    /// from `try_lock` instead — so this is carried over rather than being
    /// new coverage, just rewritten against the new `Option`-returning API.
    #[test]
    fn only_one_of_two_racing_acquirers_wins() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("race.lock");

        // A gate both threads wait on, so they really do collide rather than
        // running one after the other.
        let gate = std::sync::Arc::new(std::sync::Barrier::new(2));

        // Both threads must be started before either is waited on. Written as
        // a plain loop rather than an iterator chain on purpose: chaining
        // `.map(spawn)` straight into `.map(join)` would start one thread,
        // wait for it to finish, and only then start the second — which is
        // the exact opposite of the collision this test needs.
        let mut handles = Vec::with_capacity(2);
        for _ in 0..2 {
            let path = path.clone();
            let gate = std::sync::Arc::clone(&gate);
            handles.push(std::thread::spawn(move || {
                gate.wait();
                LockFile::try_acquire(&path, "race").ok().flatten()
            }));
        }

        // Keep the locks themselves, not just whether each attempt succeeded.
        // Holding on to them until after the count is what makes this test
        // reliable: if the winner's lock were dropped as soon as its thread
        // ended, the loser could take the freed lock and the result would
        // depend on thread timing.
        let mut results: Vec<Option<LockFile>> = Vec::with_capacity(2);
        for handle in handles {
            results.push(handle.join().unwrap());
        }
        let winners = results.iter().filter(|lock| lock.is_some()).count();

        assert_eq!(
            winners, 1,
            "exactly one of two racing acquirers may take the write lock"
        );
    }

    /// Many threads, each opening its own handle, hammering the same lock —
    /// at most one may ever hold it at the same instant. Each thread opens
    /// its own handle deliberately: `only_one_of_two_racing_acquirers_wins`
    /// already proves two *different* handles cannot both lock the file, so
    /// this is stressing that same property under sustained contention
    /// rather than proving anything new about same-process handles.
    #[test]
    fn many_threads_never_hold_the_lock_at_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("stress.lock");

        let concurrent = std::sync::Arc::new(AtomicUsize::new(0));
        let max_seen = std::sync::Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..8 {
            let path = path.clone();
            let concurrent = std::sync::Arc::clone(&concurrent);
            let max_seen = std::sync::Arc::clone(&max_seen);
            handles.push(std::thread::spawn(move || {
                for _ in 0..200 {
                    // A busy `Ok(None)` here is the expected, constant case
                    // under contention, not a failure — only a delay, so the
                    // thread just spins until its own turn comes.
                    loop {
                        if let Ok(Some(lock)) = LockFile::try_acquire(&path, "stress") {
                            let now = concurrent.fetch_add(1, Ordering::SeqCst) + 1;
                            max_seen.fetch_max(now, Ordering::SeqCst);
                            // Give another thread a chance to observe us
                            // holding it before we let go.
                            std::thread::yield_now();
                            concurrent.fetch_sub(1, Ordering::SeqCst);
                            drop(lock);
                            break;
                        }
                        std::thread::yield_now();
                    }
                }
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(
            max_seen.load(Ordering::SeqCst),
            1,
            "at most one thread may ever hold the write lock at the same time"
        );
    }

    // ── Giving the lock up ───────────────────────────────────────────────

    /// Dropping the value (rather than calling `release` explicitly) must
    /// still let the next attempt succeed — an early return or a panic that
    /// unwinds must not leave renaming blocked forever.
    #[test]
    fn dropping_the_lock_lets_the_next_attempt_succeed() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        {
            let _lock = LockFile::try_acquire(&path, "first").unwrap().unwrap();
            assert!(
                LockFile::try_acquire(&path, "second").unwrap().is_none(),
                "a second attempt must be refused while the first is still alive"
            );
        } // dropped here without calling `release`

        assert!(
            LockFile::try_acquire(&path, "second").unwrap().is_some(),
            "the lock must be available again once the first holder has been dropped"
        );
    }

    /// **Required — issue #49, finding #6.** The old design gave the lock up
    /// by deleting the file, and deleted *whatever file was at that path*
    /// when it did so — including one a second, genuinely live holder had
    /// since put there. Before this rewrite: the file the second holder
    /// wrote is gone once the first is dropped. After: it must survive,
    /// because nothing in this module ever calls `remove_file` on it.
    #[test]
    fn releasing_never_deletes_the_lock_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        let lock = LockFile::try_acquire(&path, "test").unwrap().unwrap();
        lock.release();

        assert!(path.is_file(), "release() must never delete the lock file");
    }

    /// **Required, `#[cfg(unix)]` — issue #49, finding #6.** Take the lock,
    /// have something else replace the file underneath it (standing in for
    /// a second, genuinely live holder having since taken the lock at the
    /// same path — which on Unix, where locks attach to the open file
    /// description rather than the path, is possible), then drop the first
    /// holder. The file must survive.
    ///
    /// **Rewritten (issue #49, review round) to genuinely replace the
    /// file.** The previous version of this test edited the bytes of the
    /// lock file *in place* with `std::fs::write`, which — on the same
    /// inode the first holder's handle was still attached to — proves
    /// nothing about a *replacement*. Deleting the file first and then
    /// writing a fresh one at the same path gives that path a genuinely
    /// different inode underneath the first holder's still-open handle,
    /// which is what a second live holder actually does.
    #[cfg(unix)]
    #[test]
    fn a_holder_never_deletes_a_lock_file_that_was_replaced() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        let lock = LockFile::try_acquire(&path, "test").unwrap().unwrap();

        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, "a different holder's content").unwrap();

        drop(lock);

        assert!(
            path.is_file(),
            "a holder must never delete a lock file that was replaced under it"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "a different holder's content",
            "the replacement content must survive the first holder's drop — nothing may \
             touch the file at this path once another holder has taken it over"
        );
    }

    /// **Required, `#[cfg(windows)] — cannot run or even compile-check on
    /// this development machine (macOS).** Written against the documented
    /// behaviour of the share-mode flags set in `try_acquire`: opening a file
    /// without `FILE_SHARE_DELETE` denies every other handle — including a
    /// plain `DeleteFile`/`std::fs::remove_file` — the ability to remove or
    /// replace it while the first handle stays open. This is genuinely
    /// different from Unix, where deleting a file out from under an open
    /// (even locked) handle is ordinarily possible. Confirm on a Windows CI
    /// runner before relying on this.
    #[cfg(windows)]
    #[test]
    fn the_lock_file_cannot_be_deleted_while_held() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        let lock = LockFile::try_acquire(&path, "test").unwrap().unwrap();

        let removed = std::fs::remove_file(&path);
        assert!(
            removed.is_err(),
            "Windows must refuse to delete the lock file while a deny-delete handle holds it open"
        );

        drop(lock);
    }

    // ── The two bugs that made a lock file block forever ────────────────

    /// **Required — issue #49, finding #7.** A lock file naming an unrelated
    /// process that genuinely is running must not block. Under the new
    /// design this is trivial — file content plays no part in the decision
    /// at all — but it is exactly the scenario that blocked every rename
    /// forever under the old PID-checking design once a number was reused.
    /// PID 1 is always running (`launchd`/`init`), standing in for "some
    /// unrelated program happens to reuse this number".
    #[test]
    fn a_lock_file_naming_an_unrelated_live_process_does_not_block() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");
        #[cfg(unix)]
        std::fs::write(&path, "1").unwrap();
        // PID 4 is the Windows "System" process, which is always running.
        #[cfg(windows)]
        std::fs::write(&path, "4").unwrap();

        let result = LockFile::try_acquire(&path, "test");
        assert!(
            matches!(result, Ok(Some(_))),
            "a lock file naming an unrelated live process must not block"
        );
    }

    /// **Required — issue #49, finding #16.** An empty lock file — exactly
    /// what a write that failed part-way through would leave behind under
    /// the old design — must not block forever either.
    #[test]
    fn an_empty_lock_file_does_not_block() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");
        std::fs::write(&path, "").unwrap();

        let result = LockFile::try_acquire(&path, "test");
        assert!(
            matches!(result, Ok(Some(_))),
            "an empty lock file must not block forever"
        );
    }

    // ── Telling a blocked user who holds it ─────────────────────────────

    /// **Required, new API.** A refused caller can read who holds the lock —
    /// best effort, for a message, not for a decision.
    #[test]
    fn a_refused_copy_is_told_who_holds_the_lock() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        let _first = LockFile::try_acquire(&path, "meedya scan --execute")
            .unwrap()
            .expect("first attempt must take the lock");

        let second = LockFile::try_acquire(&path, "meedya scan --execute").unwrap();
        assert!(
            second.is_none(),
            "a lock already held must refuse a second holder"
        );

        let holder = LockFile::holder(&path).expect("the info note must be readable");
        assert_eq!(holder.pid, Some(std::process::id()));
        assert_eq!(holder.purpose.as_deref(), Some("meedya scan --execute"));
        assert!(
            holder.started.is_some(),
            "a timestamp must have been recorded"
        );
    }

    /// No note has ever been written for a path nothing has taken the lock
    /// at — `holder` must say so plainly rather than fabricating an answer.
    #[test]
    fn holder_is_none_when_nothing_has_ever_held_the_lock() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        assert!(LockFile::holder(&path).is_none());
    }

    /// Once released, the note is emptied — a reader must see "nobody",
    /// not the previous holder's now-meaningless process number.
    #[test]
    fn release_clears_the_info_note_so_a_reader_sees_no_current_holder() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        let lock = LockFile::try_acquire(&path, "test").unwrap().unwrap();
        assert!(LockFile::holder(&path).is_some());

        lock.release();

        assert!(
            LockFile::holder(&path).is_none(),
            "the note must be cleared once the lock is released"
        );
    }

    // ── Messages shown to a blocked or refused user — issue #49, review
    //    round. `lock_busy_message` and `lock_unavailable_message` used to
    //    be duplicated (with drifted wording) between `crates/mm-cli` and
    //    `crates/mm-ffi`; this is the shared home both now call into. ──────

    /// The busy message must name the holder when one was recorded — the
    /// process id, what it was doing and when it started — so somebody
    /// blocked by another copy of MeedyaManager knows what they are
    /// actually waiting on.
    #[test]
    fn the_busy_message_names_the_holder_when_known() {
        let holder = LockHolder {
            pid: Some(4321),
            started: Some("2026-09-15T10:42:00Z".to_string()),
            purpose: Some("meedya scan --execute".to_string()),
        };

        let message = lock_busy_message(Some(&holder));

        assert!(
            message.contains("4321"),
            "the message must name the process id: {message}"
        );
        assert!(
            message.contains("meedya scan --execute"),
            "the message must name what the other copy was doing: {message}"
        );
        assert!(
            message.to_lowercase().contains("moving files"),
            "the message must say plainly what is happening: {message}"
        );
    }

    /// With nothing readable recorded — the note was never written, or a
    /// previous holder's write to it failed — the message must not invent
    /// a holder to name; it simply says the write is blocked.
    #[test]
    fn the_busy_message_omits_the_holder_sentence_when_unknown() {
        let message = lock_busy_message(None);

        assert!(
            message.to_lowercase().contains("moving files"),
            "the message must still say plainly what is happening: {message}"
        );
        assert!(
            !message.contains("It appears to be"),
            "with nothing readable, the message must not claim to know who holds it: {message}"
        );
    }

    /// **Fix 1 — the network-drive caveat is not a default.** It must
    /// appear only when the operating system itself says locking is not
    /// supported at all (`ErrorKind::Unsupported`) — never for an ordinary
    /// permissions problem, which has nothing to do with what kind of drive
    /// is involved. The underlying error text must still come through
    /// either way.
    #[test]
    fn the_unavailable_message_mentions_network_drives_only_when_unsupported() {
        let unsupported = std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "file locking is not supported on this filesystem",
        );
        let denied = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");

        let unsupported_message = lock_unavailable_message(&unsupported);
        let denied_message = lock_unavailable_message(&denied);

        assert!(
            unsupported_message.to_lowercase().contains("network drive"),
            "an Unsupported error must mention network drives: {unsupported_message}"
        );
        assert!(
            !denied_message.to_lowercase().contains("network drive"),
            "a plain permission error must not be blamed on a network drive: {denied_message}"
        );
        assert!(
            denied_message.contains("Permission denied"),
            "the underlying error text must still reach the user: {denied_message}"
        );
    }

    /// **Fix 1 — the harmful advice this whole fix exists to remove.** None
    /// of these messages, busy or unavailable, with or without a known
    /// holder, across every error kind, may ever tell somebody to move
    /// `MM_CONFIG_DIR`, delete or remove a file, or stop another copy of
    /// MeedyaManager mid-batch — see the doc comment on
    /// `lock_unavailable_message` for why each of those is actively wrong,
    /// not merely unhelpful.
    #[test]
    fn no_lock_message_ever_suggests_moving_settings_or_deleting_files() {
        let holder = LockHolder {
            pid: Some(1),
            started: Some("2026-09-15T10:42:00Z".to_string()),
            purpose: Some("meedya scan --execute".to_string()),
        };

        let messages = [
            lock_busy_message(Some(&holder)),
            lock_busy_message(None),
            lock_unavailable_message(&std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "not supported",
            )),
            lock_unavailable_message(&std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "denied",
            )),
            lock_unavailable_message(&std::io::Error::other("other")),
        ];

        for message in &messages {
            for forbidden in ["MM_CONFIG_DIR", "delete", "Delete", "remove", "stop it"] {
                assert!(
                    !message.contains(forbidden),
                    "no lock message may ever contain {forbidden:?}: {message}"
                );
            }
        }
    }

    // ── Crossing real process boundaries ────────────────────────────────
    //
    // Everything above proves the property within one process (multiple
    // threads, multiple handles). These two prove the two behaviours that
    // actually matter in production, across a real process boundary: a
    // holder that crashes gives the lock up, and a holder that is genuinely
    // alive keeps blocking until it is not.

    /// Env var carrying the lock path from a parent test to the child test
    /// process it spawns — read only by the two helpers immediately below,
    /// and only ever set on the child's own environment via `Command::env`,
    /// never on this process's.
    const CHILD_LOCK_PATH_ENV: &str = "MM_LOCK_TEST_CHILD_PATH";

    /// Child helper: take the lock, announce it, then abort without any
    /// orderly shutdown — standing in for a crash. A no-op unless invoked as
    /// a child (the env var is unset for every ordinary run of the suite),
    /// which is what lets this sit harmlessly in the normal test list.
    #[test]
    fn helper_child_holds_the_lock_then_aborts() {
        let Ok(path) = std::env::var(CHILD_LOCK_PATH_ENV) else {
            return;
        };
        let _lock = LockFile::try_acquire(Path::new(&path), "child-abort")
            .expect("acquiring must not error")
            .expect("child must be able to take the lock");
        println!("child-ready");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        // Never unwinds, never runs destructors — exactly like `panic =
        // "abort"` in this workspace's release profile, or a `kill -9`.
        std::process::abort();
    }

    /// Child helper: take the lock, announce it, then sit there — blocked
    /// reading its own standard input to the end — so the parent can prove
    /// a second attempt is refused while this is genuinely alive, before
    /// exiting once the parent closes that input, so the parent can then
    /// prove the lock frees the moment it does.
    ///
    /// **Rewritten (issue #49, review round) to remove a fixed sleep.** The
    /// previous version of this helper slept for a flat 5 seconds, and the
    /// test below simply waited that out via `child.wait()` regardless of
    /// how long its own checks actually took — so the test was both slower
    /// than it needed to be, and, on a slow or heavily loaded machine, only
    /// *coincidentally* long enough rather than provably so. Blocking on
    /// stdin instead means the child holds the lock for exactly as long as
    /// the parent needs it to and not an instant longer, with no wall-clock
    /// duration left to get wrong.
    #[test]
    fn helper_child_holds_the_lock_until_stdin_closes() {
        let Ok(path) = std::env::var(CHILD_LOCK_PATH_ENV) else {
            return;
        };
        let _lock = LockFile::try_acquire(Path::new(&path), "child-live")
            .expect("acquiring must not error")
            .expect("child must be able to take the lock");
        println!("child-ready");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        // Blocks until the parent drops its handle to this process's stdin
        // (see `spawn_ready_child`'s `Stdio::piped()`), at which point this
        // read returns end-of-file having read nothing — the content, if
        // any, is irrelevant, only the closing is the signal.
        use std::io::Read;
        let mut discard = String::new();
        let _ = std::io::stdin().read_to_string(&mut discard);
    }

    /// Spawn this very test binary again, filtered with `--exact` to just
    /// one child helper, and block until it prints its "ready" line — so the
    /// caller knows the child genuinely holds the lock before it does
    /// anything that depends on that.
    ///
    /// Reads and discards lines until it finds `"child-ready"` rather than
    /// reading exactly one line and comparing it. The first version of this
    /// helper did the latter, and it failed on every run: with `--nocapture`
    /// the test harness's *own* startup chatter (a blank line, then
    /// `"running 1 test"`) reaches the pipe first, so a single `read_line`
    /// always matched against that instead of the child's own output.
    fn spawn_ready_child(test_name: &str, lock_path: &Path) -> std::process::Child {
        use std::io::BufRead;

        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .arg(test_name)
            .arg("--exact")
            .arg("--nocapture")
            .env(CHILD_LOCK_PATH_ENV, lock_path)
            // Piped so a caller can signal "you may stop holding the lock
            // now" by closing this end — see
            // `helper_child_holds_the_lock_until_stdin_closes` and
            // `a_live_holder_in_another_process_blocks_and_then_frees`. The
            // abort helper never reads its stdin at all, so piping it here
            // changes nothing for that test: the pipe is simply dropped,
            // unread, the moment the child calls `process::abort()`.
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            // The harness's own end-of-run summary can still try to write
            // after we stop reading below (we only read until we see
            // "child-ready", not until the child exits) — that write then
            // fails with a broken-pipe error on the child's side. It is
            // harmless noise about a line nobody asked to see, not this
            // helper; silencing stderr keeps it out of a passing run's
            // output rather than pretending it means something.
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("failed to spawn child test process");

        let stdout = child.stdout.take().expect("child stdout must be piped");
        let mut reader = std::io::BufReader::new(stdout);
        loop {
            let mut line = String::new();
            let bytes_read = reader
                .read_line(&mut line)
                .expect("failed to read from child's stdout");
            assert_ne!(
                bytes_read, 0,
                "child's stdout closed before it ever reported ready"
            );
            if line.trim() == "child-ready" {
                break;
            }
        }

        child
    }

    /// **Required regression.** A process that aborts mid-hold must still
    /// give the lock up — this is the property that makes a crash survivable
    /// at all, and it is the one thing the old PID-checking design was
    /// actually built to handle (if by a racier route); it must not have
    /// been lost in the rewrite.
    #[test]
    fn a_process_that_aborts_while_holding_the_lock_gives_it_up() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("abort.lock");

        let mut child = spawn_ready_child(
            "state::tests::helper_child_holds_the_lock_then_aborts",
            &path,
        );
        let status = child.wait().expect("failed to wait for child");
        assert!(
            !status.success(),
            "the child was expected to abort, not exit cleanly"
        );

        let result = LockFile::try_acquire(&path, "parent");
        assert!(
            matches!(result, Ok(Some(_))),
            "the lock must be free once the aborting holder's process has ended"
        );
    }

    /// **Required regression.** A live holder in another process must
    /// genuinely block a second attempt — not merely "as long as nothing
    /// else in this process also asked for it" — and must free it the moment
    /// it exits.
    #[test]
    fn a_live_holder_in_another_process_blocks_and_then_frees() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("live.lock");

        let mut child = spawn_ready_child(
            "state::tests::helper_child_holds_the_lock_until_stdin_closes",
            &path,
        );

        assert!(
            matches!(LockFile::try_acquire(&path, "parent"), Ok(None)),
            "a second attempt must be refused while the child genuinely holds the lock"
        );

        // Tell the child to stop holding the lock by closing its stdin,
        // rather than waiting out a fixed sleep — see the doc comment on
        // `helper_child_holds_the_lock_until_stdin_closes` for why that
        // removes the timing dependence the previous version of this test
        // had.
        drop(child.stdin.take());
        child.wait().expect("failed to wait for child");

        let result = LockFile::try_acquire(&path, "parent");
        assert!(
            matches!(result, Ok(Some(_))),
            "the lock must be available once the child has exited"
        );
    }

    // ── Config directory resolution — issue #212 (P0-CONFIGDIR) ──────────

    #[test]
    fn default_lock_path_contains_meedyamanager() {
        // Same race, same fix as `config::tests::test_config_dir_returns_path_with_meedyamanager`
        // in `crates/mm-core/src/config/mod.rs` (the pointer this comment
        // used to carry — `default_state_path_contains_meedyamanager` — named
        // a test that does not exist anywhere in this crate; it has been
        // corrected to the real one). `LockFile::default_path()` reads
        // `MM_CONFIG_DIR` via `app_config_dir()`, so without this guard a
        // concurrently running test that points that variable at a tempdir
        // could make the assertion below flake.
        let _guard = crate::config::ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let path = LockFile::default_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("MeedyaManager"));
    }

    #[test]
    fn lockfile_default_path_shares_the_mm_config_dir_override() {
        // Guard against other test modules racing on the same env var.
        let _guard = crate::config::ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let tmp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("MM_CONFIG_DIR", tmp.path());
        }

        let path = LockFile::default_path();
        assert!(
            path.starts_with(tmp.path()),
            "lock file path {} does not start with override dir {}",
            path.display(),
            tmp.path().display()
        );

        unsafe {
            std::env::remove_var("MM_CONFIG_DIR");
        }
    }
}
