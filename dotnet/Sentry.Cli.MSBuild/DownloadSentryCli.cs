using System;
using System.Runtime.InteropServices;
using Microsoft.Build.Framework;
using MSBuildTask = Microsoft.Build.Utilities.Task;

namespace MSBuildTasks
{
    /// <summary>
    /// MSBuilds task to download sentry-cli.
    /// </summary>
    public class DownloadSentryCli : MSBuildTask
    {
        public string? CdnUrl { get; set; }
        public string Version { get; set; } = "latest";

        public string Architectures { get; set; } = "current";
        public string DestinationPath { get; set; } = "bin";

        public override bool Execute()
        {
            var archs = Architectures.Split(';');
            if (string.Equals(archs[0], "Current", StringComparison.InvariantCultureIgnoreCase))
            {
                var currentPlatformCliName = Get();
            }
            else if (string.Equals(archs[0], "All", StringComparison.InvariantCultureIgnoreCase))
            {

            }
            else
            {

            }

            var sentryCliDownloader = new SentryCliDownloader();
            sentryCliDownloader.DownloadForCurrentDevice(Version);

            Log.LogMessage(MessageImportance.Normal, "Downloading sentry-cli.");
            // Log.LogMessage(MessageImportance.High, $"Downloaded sentry-cli from: '{url}'");
            return true;
        }

    }
}
