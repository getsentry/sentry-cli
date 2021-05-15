using System.IO;
using System.Net.Http;
using Microsoft.Build.Framework;
using MSBuildTask = Microsoft.Build.Utilities.Task;

namespace MSBuildTasks
{
    public class DownloadSentryCli : MSBuildTask
    {
        public override bool Execute()
        {
            // https://github.com/getsentry/sentry-cli/releases
            // https://downloads.sentry-cdn.com/sentry-cli/1.64.2/sentry-cli-Darwin-universal
            var name = "sentry-cli-Darwin-universal";
            var version = "1.64.2";
            var url = $"https://downloads.sentry-cdn.com/sentry-cli/{version}/{name}";
            var client = new HttpClient();
            var cli = client.GetStreamAsync(url).GetAwaiter().GetResult();
            var fileStream = new FileStream(name, FileMode.Create);
            cli.CopyTo(fileStream);
            cli.Flush();
            fileStream.Flush();
            Log.LogMessage(MessageImportance.High, $"Getting {cli}");
            return true;
        }
    }
}
