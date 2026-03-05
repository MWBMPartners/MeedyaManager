// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — CloudProviderRow Logic Unit Tests (xUnit)
//
// Tests the CloudProviderRow view-model and provider list used by CloudPage.
// No WinUI dependency — pure C# logic.

using System.Collections.Generic;
using System.Linq;
using Xunit;
using FluentAssertions;

namespace MeedyaManager.Tests;

// ---------------------------------------------------------------------------
// Replica of CloudProviderRow (mirrors CloudPage.xaml.cs without WinUI)
// ---------------------------------------------------------------------------

internal sealed class CloudProviderRow
{
    public string Id          { get; init; } = string.Empty;
    public string Label       { get; init; } = string.Empty;
    public bool   IsConnected { get; set; }
    public string Status      { get; set; } = "Not Connected";
    public bool   IsStub      { get; init; }
}

internal static class CloudProviderList
{
    public static List<CloudProviderRow> Default() =>
    [
        new() { Id = "onedrive",    Label = "OneDrive",     IsStub = false, Status = "Not Connected" },
        new() { Id = "googledrive", Label = "Google Drive", IsStub = false, Status = "Not Connected" },
        new() { Id = "dropbox",     Label = "Dropbox",      IsStub = false, Status = "Not Connected" },
        new() { Id = "mega",        Label = "MEGA",         IsStub = true,  Status = "Coming Soon"   },
        new() { Id = "icloud",      Label = "iCloud Drive", IsStub = true,  Status = "macOS only"    },
    ];
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

public class CloudProviderRowTests
{
    [Fact]
    public void DefaultList_Should_Have5Entries()
    {
        CloudProviderList.Default().Should().HaveCount(5);
    }

    [Fact]
    public void DefaultList_Should_Have3ConcreteProviders()
    {
        CloudProviderList.Default().Count(p => !p.IsStub).Should().Be(3);
    }

    [Fact]
    public void DefaultList_Should_Have2StubProviders()
    {
        CloudProviderList.Default().Count(p => p.IsStub).Should().Be(2);
    }

    [Fact]
    public void AllProviders_Should_StartNotConnected()
    {
        var concrete = CloudProviderList.Default().Where(p => !p.IsStub);
        concrete.Should().OnlyContain(p => !p.IsConnected);
    }

    [Fact]
    public void OneDrive_Should_BeFirst()
    {
        CloudProviderList.Default()[0].Id.Should().Be("onedrive");
    }

    [Fact]
    public void Mega_Should_BeStub()
    {
        var mega = CloudProviderList.Default().First(p => p.Id == "mega");
        mega.IsStub.Should().BeTrue();
    }

    [Fact]
    public void ICloud_Should_BeStub()
    {
        var icloud = CloudProviderList.Default().First(p => p.Id == "icloud");
        icloud.IsStub.Should().BeTrue();
    }

    [Fact]
    public void AllIds_Should_BeUnique()
    {
        var ids = CloudProviderList.Default().Select(p => p.Id).ToList();
        ids.Should().OnlyHaveUniqueItems();
    }

    [Fact]
    public void Toggle_Should_MarkAsConnected()
    {
        var p = new CloudProviderRow { Id = "onedrive", Label = "OneDrive", IsStub = false };
        p.IsConnected = !p.IsConnected;
        p.IsConnected.Should().BeTrue();
    }

    [Fact]
    public void Toggle_Should_MarkAsDisconnected()
    {
        var p = new CloudProviderRow { Id = "dropbox", Label = "Dropbox", IsConnected = true, IsStub = false };
        p.IsConnected = !p.IsConnected;
        p.IsConnected.Should().BeFalse();
    }

    [Fact]
    public void Status_Should_BeUpdatable()
    {
        var p = new CloudProviderRow { Id = "onedrive", Label = "OneDrive", Status = "Not Connected", IsStub = false };
        p.Status = "Synced";
        p.Status.Should().Be("Synced");
    }

    [Fact]
    public void StubProvider_Should_NotBeConnected()
    {
        var stubs = CloudProviderList.Default().Where(p => p.IsStub);
        stubs.Should().OnlyContain(p => !p.IsConnected);
    }
}
