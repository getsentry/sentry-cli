const PR_NUMBER = danger.github.pr.number;
const PR_URL = danger.github.pr.html_url;
const PR_LINK = `[#${PR_NUMBER}](${PR_URL})`;

function getCleanTitle() {
  const title = danger.github.pr.title;
  return title.split(": ").slice(-1)[0].trim().replace(/\.+$/, "");
}

function getChangelogDetails() {
  return `
<details>
<summary><b>Instructions and example for changelog</b></summary>

Please add an entry to \`CHANGELOG.md\` to the "Unreleased" section under the following heading with
a link to this PR (consider a more descriptive message than the suggestion):

\`\`\`md
- ${getCleanTitle()}. (${PR_LINK})
\`\`\`

If an "Unreleased" section doesn't exist, please add one in above the section for the latest 
release.

If creating a changelog entry is not applicable to your change, you can opt out by adding 
_#skip-changelog_ to the PR description.

</details>
`;
}

async function containsChangelog(path) {
  const contents = await danger.github.utils.fileContents(path);
  return contents.includes(PR_LINK);
}

async function checkChangelog() {
  const skipChangelog =
    danger.github && (danger.github.pr.body + "").includes("#skip-changelog");

  if (skipChangelog) {
    return;
  }

  const hasChangelog = await containsChangelog("CHANGELOG.md");

  if (!hasChangelog) {
    fail("Please consider adding a changelog entry for the next release.");
    markdown(getChangelogDetails());
  }
}