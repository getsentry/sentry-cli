using System;
using System.IO;
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Build.Framework;
using MSBuildTask = Microsoft.Build.Utilities.Task;

namespace MSBuildTasks
{
    public class SentryCliDownloader
    {
        private readonly HttpMessageHandler? _handler;
        public string CdnUrl { get; set; } = "https://downloads.sentry-cdn.com/sentry-cli";

        public SentryCliDownloader() : this(null)
        { }

        public SentryCliDownloader(HttpMessageHandler? handler)
            => _handler = handler;

        public async Task DownloadForCurrentDevice(CancellationToken token, string? version = null)
        {
            version ??= await FindLatestRelease(token).ConfigureAwait(false);
            var executableName = GetSentryCliName();
            var url = GetUrl(version, executableName);
            var client = GetHttpClient();
            var cli = client.GetStreamAsync(url).GetAwaiter().GetResult();
            var fileStream = new FileStream(executableName, FileMode.Create);
            await cli.CopyToAsync(fileStream);
            cli.Flush();
            fileStream.Flush();
        }

        public string GetUrl(string version, string executableName)
            => $"{CdnUrl}/{version}/{executableName}";

        public bool Download()
        {
            return true;
        }

        private HttpClient GetHttpClient() =>
            _handler is not null
                ? new HttpClient(_handler)
                : new HttpClient();

        public async Task<string> FindLatestRelease(CancellationToken token)
        {
            const string sentryCliReleases = "https://api.github.com/repos/getsentry/sentry-cli/releases/latest";

            var client = GetHttpClient();
            var result = await client.GetAsync(sentryCliReleases, token)
                .ConfigureAwait(false);
            result.EnsureSuccessStatusCode();
            var content = await result.Content.ReadAsStringAsync();
            var match = Regex.Match(content, "\"tag_name\": \"([\\d\\.]+)\"");
            if (!match.Success)
            {
                throw new InvalidOperationException($"Couldn't find the latest release in: {sentryCliReleases}");
            }

            return match.Groups[1].Value;
        }

        public static string GetSentryCliName(Architecture? architecture = null, OSPlatform? platform = null)
        {
            var suffix = RuntimeInformation.IsOSPlatform(OSPlatform.Linux) || platform == OSPlatform.Linux
                ? $"Linux-{GetArchitectureString()}"
                : RuntimeInformation.IsOSPlatform(OSPlatform.Windows) || platform == OSPlatform.Windows
                    ? $"Windows-{GetArchitectureString()}.exe"
                    : RuntimeInformation.IsOSPlatform(OSPlatform.OSX) || platform == OSPlatform.OSX
                        ? "Darwin-universal"
                        : throw new NotSupportedException("Only Windows, macOS and Linux is supported");

            return $"sentry-cli-{suffix}";

            string GetArchitectureString()
            {
                return (architecture ?? RuntimeInformation.OSArchitecture) switch
                {
                    Architecture.Arm => throw new NotSupportedException("Arm32 is not supported"),
                    Architecture.Arm64 => "aarch64",
                    Architecture.X64 => "x86_64",
                    Architecture.X86 => "i686",
                    _ => throw new ArgumentOutOfRangeException()
                };
            }
        }

    }
}
