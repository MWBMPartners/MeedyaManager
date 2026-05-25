// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Server View (M10)
//
// SwiftUI view for the media server configuration and control tab.
// Provides a GNOME-style preferences form for all server settings
// and a start/stop control with an access log viewer.

import SwiftUI

// ---------------------------------------------------------------------------
// ServerModel — observable state for the server panel
// ---------------------------------------------------------------------------

/// Server status discriminant.
enum ServerStatus: Equatable {
    /// Server is not running
    case stopped
    /// Server is starting up
    case starting
    /// Server is running (shows bind address)
    case running(address: String)
    /// Server encountered an error
    case error(message: String)

    /// Human-readable status string
    var displayText: String {
        switch self {
        case .stopped:               return "Stopped"
        case .starting:              return "Starting…"
        case .running(let address):  return "Running — \(address)"
        case .error(let message):    return "Error: \(message)"
        }
    }

    /// `true` when the server is active
    var isRunning: Bool {
        if case .running = self { return true }
        return false
    }
}

/// Observable model for the Server tab.
/// @MainActor: state is read by SwiftUI views and mutated by user actions;
/// isolating to MainActor satisfies Swift 6 strict concurrency without adding
/// capture-list ceremony at every call site (matches CloudModel/ExportModel
/// pattern).
@MainActor
@Observable
final class ServerModel {
    // ── Network settings ──────────────────────────────────────────────────
    var bindAddress: String = "0.0.0.0"
    var port: String        = "8443"

    // ── TLS settings ──────────────────────────────────────────────────────
    var tlsCertPath: String = ""
    var tlsKeyPath:  String = ""
    var noTls:       Bool   = false

    // ── Authentication ────────────────────────────────────────────────────
    var jwtSecret:       String = ""
    var jwtExpirySecs:   String = "86400"

    // ── CORS ──────────────────────────────────────────────────────────────
    var corsOrigins: String = ""

    // ── State ─────────────────────────────────────────────────────────────
    var status:   ServerStatus = .stopped
    var logLines: [String]     = []
    var isLoading: Bool        = false

    /// Effective bind address string (`host:port`)
    var bindAddr: String {
        "\(bindAddress):\(port)"
    }

    /// Validate the current configuration.
    /// Returns `nil` if valid; returns an error message if invalid.
    var validationError: String? {
        if jwtSecret.trimmingCharacters(in: .whitespaces).isEmpty {
            return "JWT secret is required. Set it here or use the MM_JWT_SECRET environment variable."
        }
        if !noTls {
            if tlsCertPath.trimmingCharacters(in: .whitespaces).isEmpty {
                return "TLS certificate path is required (or enable 'Disable TLS' for local development)."
            }
            if tlsKeyPath.trimmingCharacters(in: .whitespaces).isEmpty {
                return "TLS private key path is required."
            }
        }
        return nil
    }

    // ── Actions ──────────────────────────────────────────────────────────

    /// Start the media server.
    func startServer() {
        guard validationError == nil else {
            appendLog("ERROR: \(validationError!)")
            status = .error(message: validationError!)
            return
        }

        status = .starting
        isLoading = true
        appendLog("Starting MeedyaManager media server…")
        appendLog("Binding to \(noTls ? "http" : "https")://\(bindAddr)")
        appendLog("JWT expiry: \(jwtExpirySecs) seconds")

        // M10 stub: simulate server startup latency. Swift 6 idiom (Task + sleep)
        // replaces nested DispatchQueue patterns that trigger sending-self errors.
        Task { [weak self] in
            try? await Task.sleep(for: .seconds(1.2))
            guard let self else { return }
            self.isLoading = false
            self.status    = .running(address: "\(self.noTls ? "http" : "https")://\(self.bindAddr)")
            self.appendLog("Server ready. Listening on \(self.noTls ? "http" : "https")://\(self.bindAddr)")
            self.appendLog("Routes: GET /health, POST /auth/login, GET /api/library, GET /stream/:id")
        }
    }

    /// Stop the media server.
    func stopServer() {
        appendLog("Stopping server…")
        isLoading = true

        Task { [weak self] in
            try? await Task.sleep(for: .seconds(0.5))
            guard let self else { return }
            self.isLoading = false
            self.status    = .stopped
            self.appendLog("Server stopped.")
        }
    }

    /// Show the route table in the log.
    func showRoutes() {
        let routes = [
            ("GET",  "/health",          "Liveness probe — no auth required"),
            ("POST", "/auth/login",       "Authenticate → JWT"),
            ("GET",  "/api/library",      "List media files (paginated)"),
            ("GET",  "/api/library/:id",  "Single file metadata by ID"),
            ("GET",  "/api/search",       "Search by title/artist/album"),
            ("GET",  "/stream/:id",       "Stream media file (Range requests)"),
            ("GET",  "/api/export/status","Export status (Admin only)"),
            ("GET",  "/api/server/info",  "Server info + version (Admin only)"),
        ]
        appendLog("─── Route Table ─────────────────────────────────")
        for (method, path, desc) in routes {
            appendLog(String(format: "%-5s %-25s — %s", method, path, desc))
        }
    }

    /// Clear the log and reset status to stopped.
    func clearLog() {
        logLines = []
        if case .error = status { status = .stopped }
    }

    /// Append a timestamped line to the log (max 200 lines).
    private func appendLog(_ line: String) {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm:ss"
        let ts = formatter.string(from: Date())
        logLines.insert("[\(ts)] \(line)", at: 0)
        if logLines.count > 200 {
            logLines = Array(logLines.prefix(200))
        }
    }
}

// ---------------------------------------------------------------------------
// ServerView — SwiftUI layout
// ---------------------------------------------------------------------------

/// The Server tab — configure and control the HTTPS media server.
struct ServerView: View {
    @State private var model = ServerModel()

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {

                // ── Network ────────────────────────────────────────────────
                SettingsGroupServer(title: "Network",
                                    subtitle: "Bind address and port") {
                    LabeledContent("Bind address") {
                        TextField("0.0.0.0", text: $model.bindAddress)
                            .textFieldStyle(.roundedBorder)
                            .frame(maxWidth: 200)
                            .help("Use 0.0.0.0 for all interfaces or 127.0.0.1 for loopback only.")
                            .accessibilityLabel("Bind address")
                            .accessibilityHint("Use 0.0.0.0 for all interfaces or 127.0.0.1 for loopback only")
                    }
                    LabeledContent("Port") {
                        TextField("8443", text: $model.port)
                            .textFieldStyle(.roundedBorder)
                            .frame(maxWidth: 80)
                            .help("TCP port for HTTPS connections. Ports below 1024 require elevated privileges.")
                            .accessibilityLabel("Port")
                            .accessibilityHint("TCP port for HTTPS connections. Ports below 1024 require elevated privileges.")
                    }
                }

                // ── TLS ────────────────────────────────────────────────────
                SettingsGroupServer(title: "TLS / HTTPS",
                                    subtitle: "Certificate and private key") {
                    LabeledContent("Certificate (PEM)") {
                        TextField("/etc/ssl/cert.pem", text: $model.tlsCertPath)
                            .textFieldStyle(.roundedBorder)
                            .help("Path to PEM-encoded TLS certificate file.")
                            .accessibilityLabel("TLS certificate path")
                            .accessibilityHint("Enter the file path to your PEM-encoded TLS certificate")
                    }
                    LabeledContent("Private key (PEM)") {
                        TextField("/etc/ssl/key.pem", text: $model.tlsKeyPath)
                            .textFieldStyle(.roundedBorder)
                            .help("Path to PEM-encoded TLS private key file.")
                            .accessibilityLabel("TLS private key path")
                            .accessibilityHint("Enter the file path to your PEM-encoded TLS private key")
                    }
                    Toggle("Disable TLS (HTTP only — development use)", isOn: $model.noTls)
                        .help("Use plain HTTP instead of HTTPS. Not recommended for production.")
                        .accessibilityHint("Switches to plain HTTP. Not recommended for production deployments.")
                }

                // ── Authentication ────────────────────────────────────────
                SettingsGroupServer(title: "Authentication",
                                    subtitle: "JWT signing secret and token lifetime") {
                    LabeledContent("JWT secret") {
                        SecureField("or set MM_JWT_SECRET env var", text: $model.jwtSecret)
                            .textFieldStyle(.roundedBorder)
                            .help("Secret key used to sign JWT tokens. Minimum 16 characters.")
                            .accessibilityLabel("JWT secret")
                            .accessibilityHint("Secret key used to sign authentication tokens. Minimum 16 characters.")
                    }
                    LabeledContent("Token expiry (s)") {
                        TextField("86400", text: $model.jwtExpirySecs)
                            .textFieldStyle(.roundedBorder)
                            .frame(maxWidth: 100)
                            .help("How long issued JWTs remain valid. Default: 86400 (24 hours).")
                            .accessibilityLabel("Token expiry in seconds")
                            .accessibilityHint("How long issued tokens remain valid. Default is 86400, which is 24 hours.")
                    }
                }

                // ── CORS ──────────────────────────────────────────────────
                SettingsGroupServer(title: "CORS",
                                    subtitle: "Allowed cross-origin request origins") {
                    LabeledContent("Allowed origins") {
                        TextField("https://app.example.com, https://local.dev:3000",
                                  text: $model.corsOrigins)
                            .textFieldStyle(.roundedBorder)
                            .help("Comma-separated list of allowed origins. Leave empty to deny cross-origin requests.")
                            .accessibilityLabel("CORS allowed origins")
                            .accessibilityHint("Comma-separated list of allowed cross-origin request origins. Leave empty to deny all cross-origin requests.")
                    }
                }

                // ── Control ───────────────────────────────────────────────
                SettingsGroupServer(title: "Server Control", subtitle: nil) {
                    // Validation error (if any)
                    if let err = model.validationError, !model.status.isRunning {
                        Label(err, systemImage: "exclamationmark.triangle")
                            .foregroundStyle(.orange)
                            .font(.caption)
                    }

                    // Status
                    HStack {
                        // Decorative dot — status announced via text live region
                        Circle()
                            .fill(statusColor)
                            .frame(width: 10, height: 10)
                            .accessibilityHidden(true)
                        Text(model.status.displayText)
                            .font(.callout)
                            .foregroundStyle(.secondary)
                            .accessibilityLabel("Server status: \(model.status.displayText)")
                            // accessibilityLiveRegion is a UIAccessibility/AppKit property,
                            // not a SwiftUI View modifier. Status changes are surfaced via
                            // accessibilityLabel above; consider AccessibilityNotification
                            // .Announcement.post(...) if explicit announcement is desired.
                    }

                    // Buttons
                    HStack(spacing: 12) {
                        Button(action: { model.startServer() }) {
                            Label("Start Server", systemImage: "play.fill")
                        }
                        .buttonStyle(.borderedProminent)
                        .disabled(model.status.isRunning || model.isLoading)
                        .accessibilityLabel("Start server")
                        .accessibilityHint("Starts the HTTPS media server with the current configuration")

                        Button(action: { model.stopServer() }) {
                            Label("Stop Server", systemImage: "stop.fill")
                        }
                        .buttonStyle(.bordered)
                        .tint(.red)
                        .disabled(!model.status.isRunning || model.isLoading)
                        .accessibilityLabel("Stop server")
                        .accessibilityHint("Stops the running media server")

                        Button(action: { model.showRoutes() }) {
                            Label("Show Routes", systemImage: "list.bullet")
                        }
                        .buttonStyle(.bordered)
                        .accessibilityLabel("Show routes")
                        .accessibilityHint("Displays the API route table in the access log")
                    }
                }

                // ── Access Log ───────────────────────────────────────────
                SettingsGroupServer(title: "Access Log", subtitle: nil) {
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 2) {
                            ForEach(model.logLines.indices, id: \.self) { idx in
                                Text(model.logLines[idx])
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundStyle(.secondary)
                                    .textSelection(.enabled)
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(8)
                    }
                    .frame(height: 180)
                    .background(Color(.textBackgroundColor))
                    .clipShape(RoundedRectangle(cornerRadius: 6))
                    .overlay(
                        RoundedRectangle(cornerRadius: 6)
                            .stroke(Color.secondary.opacity(0.2), lineWidth: 1)
                    )

                    HStack {
                        Spacer()
                        Button("Clear Log") { model.clearLog() }
                            .buttonStyle(.bordered)
                            .accessibilityLabel("Clear access log")
                            .accessibilityHint("Removes all log entries and resets any error state")
                    }
                }
            }
            .padding()
        }
        .navigationTitle("Server")
    }

    /// Traffic-light colour for the current server status.
    private var statusColor: Color {
        switch model.status {
        case .stopped:  return .gray
        case .starting: return .yellow
        case .running:  return .green
        case .error:    return .red
        }
    }
}

// ---------------------------------------------------------------------------
// SettingsGroupServer — private preferences group for this view
// (avoids name clash with SettingsGroup in SettingsView.swift)
// ---------------------------------------------------------------------------

private struct SettingsGroupServer<Content: View>: View {
    let title: String
    let subtitle: String?
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.headline)
                if let sub = subtitle {
                    Text(sub).font(.caption).foregroundStyle(.secondary)
                }
            }
            VStack(alignment: .leading, spacing: 10) {
                content()
            }
            .padding(12)
            .background(Color(.controlBackgroundColor))
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(Color.secondary.opacity(0.15), lineWidth: 1)
            )
        }
    }
}

#Preview {
    ServerView()
        .frame(width: 720, height: 700)
}
