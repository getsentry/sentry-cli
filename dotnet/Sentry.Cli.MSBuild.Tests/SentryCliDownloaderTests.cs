using System;
using System.Net;
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using MSBuildTasks;
using Xunit;

namespace Sentry.Cli.MSBuild.Tests
{
    public class SentryCliDownloaderTests
    {
        private class Fixture
        {
            public MockHttpMessageHandler MockHttpMessageHandler { get; set; } = new(
                (_, _) => Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK)));

            public SentryCliDownloader GetSut() => new(MockHttpMessageHandler);
        }

        private readonly Fixture _fixture = new();

        [Fact]
        public void CdnUrl_Defaults_SentryCdn()
        {
            var sut = _fixture.GetSut();
            Assert.Equal("https://downloads.sentry-cdn.com/sentry-cli", sut.CdnUrl);
        }

        [Fact]
        public async Task FindLatestRelease()
        {
            _fixture.MockHttpMessageHandler = new MockHttpMessageHandler((_, _)
                => Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK)
                {
                    Content = new StringContent(_sampleJsonResponseLatestRelease)
                }));

            var sut = _fixture.GetSut();
            var result = await sut.FindLatestRelease(CancellationToken.None)
                .ConfigureAwait(false);
            Assert.Equal("1.64.2", result);
        }

        [Fact]
        public void Download_BadHttpRequest_ThrowsHttpRequestException()
        {
            _fixture.MockHttpMessageHandler = new MockHttpMessageHandler((_, _)
                => Task.FromResult(new HttpResponseMessage(HttpStatusCode.ServiceUnavailable)));

            var sut = _fixture.GetSut();
            // Assert.Throws<HttpRequestException>(() => sut.Download());
        }

        [Fact]
        public void GetUrl_ChangedCdn_RequestedPlatform()
        {
            const string version = "1.1.1";
            const string testCdn = "http://localhost";
            var sut = _fixture.GetSut();
            sut.CdnUrl = testCdn;
            Assert.Equal($"{testCdn}/{version}/sentry-cli-Linux-x86_64",
                sut.GetUrl(version, SentryCliDownloader.GetSentryCliName(Architecture.X64, OSPlatform.Linux)));
        }

        [Fact]
        public void GetUrl_DefaultCdn_RequestedPlatform()
        {
            const string version = "1.1.1";
            var sut = _fixture.GetSut();
            Assert.Equal($"https://downloads.sentry-cdn.com/sentry-cli/{version}/test",
                sut.GetUrl(version, "test"));
        }

        [Fact]
        public void GetSentryCliName_RequestedPlatform()
        {
            Assert.Equal("sentry-cli-Linux-x86_64", SentryCliDownloader.GetSentryCliName(Architecture.X64, OSPlatform.Linux));
            Assert.Equal("sentry-cli-Linux-i686", SentryCliDownloader.GetSentryCliName(Architecture.X86, OSPlatform.Linux));
            Assert.Equal("sentry-cli-Darwin-universal", SentryCliDownloader.GetSentryCliName(Architecture.X86, OSPlatform.OSX));
            Assert.Equal("sentry-cli-Darwin-universal", SentryCliDownloader.GetSentryCliName(Architecture.Arm64, OSPlatform.OSX));
            Assert.Equal("sentry-cli-Darwin-universal", SentryCliDownloader.GetSentryCliName(Architecture.Arm, OSPlatform.OSX));
            Assert.Equal("sentry-cli-Darwin-universal", SentryCliDownloader.GetSentryCliName(Architecture.X64, OSPlatform.OSX));
            Assert.Equal("sentry-cli-Windows-x86_64.exe", SentryCliDownloader.GetSentryCliName(Architecture.X64, OSPlatform.Windows));
            Assert.Equal("sentry-cli-Windows-i686.exe", SentryCliDownloader.GetSentryCliName(Architecture.X86, OSPlatform.Windows));
        }

        [Fact]
        public void GetSentryCliName_CurrentPlatform()
        {
            var result = SentryCliDownloader.GetSentryCliName();
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            {
                switch (RuntimeInformation.OSArchitecture)
                {
                    case Architecture.X64:
                        Assert.Equal("sentry-cli-Linux-x86_64", result);
                        break;
                    case Architecture.X86:
                        Assert.Equal("sentry-cli-Linux-i686", result);
                        break;
                    default:
                        Assert.False(true, $"Cannot verify '{result}' on this platform.");
                        break;
                }
            }
            else if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            {
                Assert.Equal("sentry-cli-Darwin-universal", result);
            }
            else if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                switch (RuntimeInformation.OSArchitecture)
                {
                    case Architecture.X64:
                        Assert.Equal("sentry-cli-Windows-x86_64.exe", result);
                        break;
                    case Architecture.X86:
                        Assert.Equal("sentry-cli-Windows-i686.exe", result);
                        break;
                    default:
                        Assert.False(true, $"Cannot verify '{result}' on this platform.");
                        break;
                }
            }
            else
            {
                Assert.False(true, $"Cannot verify '{result}' on this platform.");
            }
        }

        private const string _sampleJsonResponseLatestRelease = @"{
  ""url"": ""https://api.github.com/repos/getsentry/sentry-cli/releases/42693461"",
  ""assets_url"": ""https://api.github.com/repos/getsentry/sentry-cli/releases/42693461/assets"",
  ""upload_url"": ""https://uploads.github.com/repos/getsentry/sentry-cli/releases/42693461/assets{?name,label}"",
  ""html_url"": ""https://github.com/getsentry/sentry-cli/releases/tag/1.64.2"",
  ""id"": 42693461,
  ""author"": {
    ""login"": ""getsentry-bot"",
    ""id"": 10587625,
    ""node_id"": ""MDQ6VXNlcjEwNTg3NjI1"",
    ""avatar_url"": ""https://avatars.githubusercontent.com/u/10587625?v=4"",
    ""gravatar_id"": """",
    ""url"": ""https://api.github.com/users/getsentry-bot"",
    ""html_url"": ""https://github.com/getsentry-bot"",
    ""followers_url"": ""https://api.github.com/users/getsentry-bot/followers"",
    ""following_url"": ""https://api.github.com/users/getsentry-bot/following{/other_user}"",
    ""gists_url"": ""https://api.github.com/users/getsentry-bot/gists{/gist_id}"",
    ""starred_url"": ""https://api.github.com/users/getsentry-bot/starred{/owner}{/repo}"",
    ""subscriptions_url"": ""https://api.github.com/users/getsentry-bot/subscriptions"",
    ""organizations_url"": ""https://api.github.com/users/getsentry-bot/orgs"",
    ""repos_url"": ""https://api.github.com/users/getsentry-bot/repos"",
    ""events_url"": ""https://api.github.com/users/getsentry-bot/events{/privacy}"",
    ""received_events_url"": ""https://api.github.com/users/getsentry-bot/received_events"",
    ""type"": ""User"",
    ""site_admin"": false
  },
  ""node_id"": ""MDc6UmVsZWFzZTQyNjkzNDYx"",
  ""tag_name"": ""1.64.2"",
  ""target_commitish"": ""master"",
  ""name"": ""sentry-cli 1.64.2"",
  ""draft"": false,
  ""prerelease"": false,
  ""created_at"": ""2021-05-10T09:52:29Z"",
  ""published_at"": ""2021-05-10T09:52:30Z"",
  ""assets"": [
    {
      ""url"": ""https://api.github.com/repos/getsentry/sentry-cli/releases/assets/36725515"",
      ""id"": 36725515,
      ""node_id"": ""MDEyOlJlbGVhc2VBc3NldDM2NzI1NTE1"",
      ""name"": ""sentry-cli-Darwin-arm64"",
      ""label"": """",
      ""uploader"": {
        ""login"": ""getsentry-bot"",
        ""id"": 10587625,
        ""node_id"": ""MDQ6VXNlcjEwNTg3NjI1"",
        ""avatar_url"": ""https://avatars.githubusercontent.com/u/10587625?v=4"",
        ""gravatar_id"": """",
        ""url"": ""https://api.github.com/users/getsentry-bot"",
        ""html_url"": ""https://github.com/getsentry-bot"",
        ""followers_url"": ""https://api.github.com/users/getsentry-bot/followers"",
        ""following_url"": ""https://api.github.com/users/getsentry-bot/following{/other_user}"",
        ""gists_url"": ""https://api.github.com/users/getsentry-bot/gists{/gist_id}"",
        ""starred_url"": ""https://api.github.com/users/getsentry-bot/starred{/owner}{/repo}"",
        ""subscriptions_url"": ""https://api.github.com/users/getsentry-bot/subscriptions"",
        ""organizations_url"": ""https://api.github.com/users/getsentry-bot/orgs"",
        ""repos_url"": ""https://api.github.com/users/getsentry-bot/repos"",
        ""events_url"": ""https://api.github.com/users/getsentry-bot/events{/privacy}"",
        ""received_events_url"": ""https://api.github.com/users/getsentry-bot/received_events"",
        ""type"": ""User"",
        ""site_admin"": false
      },
      ""content_type"": ""application/octet-stream"",
      ""state"": ""uploaded"",
      ""size"": 10443047,
      ""download_count"": 281,
      ""created_at"": ""2021-05-10T09:52:30Z"",
      ""updated_at"": ""2021-05-10T09:52:31Z"",
      ""browser_download_url"": ""https://github.com/getsentry/sentry-cli/releases/download/1.64.2/sentry-cli-Darwin-arm64""
    },
  ],
  ""tarball_url"": ""https://api.github.com/repos/getsentry/sentry-cli/tarball/1.64.2"",
  ""zipball_url"": ""https://api.github.com/repos/getsentry/sentry-cli/zipball/1.64.2"",
  ""body"": ""* ref: Rely on spawn process error for detecting command presence (#958)""
}";
    }

    internal class MockHttpMessageHandler : HttpMessageHandler
    {
        private readonly Func<HttpRequestMessage, CancellationToken, Task<HttpResponseMessage>> _handler;

        public MockHttpMessageHandler(
            Func<HttpRequestMessage, CancellationToken, Task<HttpResponseMessage>> handler)
            => _handler = handler;

        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
            => _handler(request, cancellationToken);
    }
}
