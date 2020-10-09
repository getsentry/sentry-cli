'use strict';

const helper = require('../helper');

/**
 * Default arguments for the `--ignore` option.
 * @type {string[]}
 */
const DEFAULT_IGNORE = ['node_modules'];

/**
 * Schema for the `upload-sourcemaps` command.
 * @type {OptionsSchema}
 */
const SOURCEMAPS_SCHEMA = require('./options/uploadSourcemaps');

/**
 * Schema for the `deploys new` command.
 * @type {OptionsSchema}
 */
const DEPLOYS_SCHEMA = require('./options/deploys');

/**
 * Manages releases and release artifacts on Sentry.
 * @namespace SentryReleases
 */
class Releases {
  /**
   * Creates a new `Releases` instance.
   *
   * @param {Object} [options] More options to pass to the CLI
   */
  constructor(options) {
    this.options = options || {};
    if (typeof this.options.configFile === 'string') {
      this.configFile = this.options.configFile;
    }
    delete this.options.configFile;
  }

  /**
   * Registers a new release with sentry.
   *
   * The given release name should be unique and deterministic. It can later be used to
   * upload artifacts, such as source maps.
   *
   * @param {string} release Unique name of the new release.
   * @param {object} options A set of options when creating a release.
   * @param {array} options.projects The list of project slugs for a release.
   * @returns {Promise} A promise that resolves when the release has been created.
   * @memberof SentryReleases
   */
  new(release, options) {
    const args = ['releases', 'new', release].concat(helper.getProjectFlagsFromOptions(options));
    return this.execute(args, null);
  }

  /**
   * Specifies the set of commits covered in this release.
   *
   * @param {string} release Unique name of the release
   * @param {object} options A set of options to configure the commits to include
   * @param {string} options.repo The full repo name as defined in Sentry
   * @param {boolean} options.auto Automatically choose the associated commit (uses
   * the current commit). Overrides other options.
   * @param {string} options.commit The current (last) commit in the release.
   * @param {string} options.previousCommit The commit before the beginning of this
   * release (in other words, the last commit of the previous release). If omitted,
   * this will default to the last commit of the previous release in Sentry. If there
   * was no previous release, the last 10 commits will be used.
   * @returns {Promise} A promise that resolves when the commits have been associated
   * @memberof SentryReleases
   */
  setCommits(release, options) {
    if (!options || (!options.auto && (!options.repo || !options.commit))) {
      throw new Error('options.auto, or options.repo and options.commit must be specified');
    }

    let commitFlags = [];

    if (options.auto) {
      commitFlags = ['--auto'];
    } else if (options.previousCommit) {
      commitFlags = ['--commit', `${options.repo}@${options.previousCommit}..${options.commit}`];
    } else {
      commitFlags = ['--commit', `${options.repo}@${options.commit}`];
    }

    return this.execute(['releases', 'set-commits', release].concat(commitFlags));
  }

  /**
   * Marks this release as complete. This should be called once all artifacts has been
   * uploaded.
   *
   * @param {string} release Unique name of the release.
   * @returns {Promise} A promise that resolves when the release has been finalized.
   * @memberof SentryReleases
   */
  finalize(release) {
    return this.execute(['releases', 'finalize', release], null);
  }

  /**
   * Creates a unique, deterministic version identifier based on the project type and
   * source files. This identifier can be used as release name.
   *
   * @returns {Promise.<string>} A promise that resolves to the version string.
   * @memberof SentryReleases
   */
  proposeVersion() {
    return this.execute(['releases', 'propose-version'], null).then(
      version => version && version.trim()
    );
  }

  /**
   * Scans the given include folders for JavaScript source maps and uploads them to the
   * specified release for processing.
   *
   * The options require an `include` array, which is a list of directories to scan.
   * Additionally, it supports to ignore certain files, validate and preprocess source
   * maps and define a URL prefix.
   *
   * @example
   * await cli.releases.uploadSourceMaps(cli.releases.proposeVersion(), {
   *   // required options:
   *   include: ['build'],
   *
   *   // default options:
   *   ignore: ['node_modules'],  // globs for files to ignore
   *   ignoreFile: null,          // path to a file with ignore rules
   *   rewrite: false,            // preprocess sourcemaps before uploading
   *   sourceMapReference: true,  // add a source map reference to source files
   *   stripPrefix: [],           // remove certain prefices from filenames
   *   stripCommonPrefix: false,  // guess common prefices to remove from filenames
   *   validate: false,           // validate source maps and cancel the upload on error
   *   urlPrefix: '',             // add a prefix source map urls after stripping them
   *   urlSuffix: '',             // add a suffix source map urls after stripping them
   *   ext: ['js', 'map', 'jsbundle', 'bundle'],  // override file extensions to scan for
   *   projects: ['node']        // provide a list of projects
   * });
   *
   * @param {string} release Unique name of the release.
   * @param {object} options Options to configure the source map upload.
   * @returns {Promise} A promise that resolves when the upload has completed successfully.
   * @memberof SentryReleases
   */
  uploadSourceMaps(release, options) {
    if (!options || !options.include) {
      throw new Error('options.include must be a vaild path(s)');
    }

    const uploads = options.include.map(sourcemapPath => {
      const newOptions = { ...options };
      if (!newOptions.ignoreFile && !newOptions.ignore) {
        newOptions.ignore = DEFAULT_IGNORE;
      }

      const args = ['releases']
        .concat(helper.getProjectFlagsFromOptions(options))
        .concat(['files', release, 'upload-sourcemaps', sourcemapPath]);
      return this.execute(helper.prepareCommand(args, SOURCEMAPS_SCHEMA, newOptions), true);
    });

    return Promise.all(uploads);
  }

  /**
   * List all deploys for a given release.
   *
   * @param {string} release Unique name of the release.
   * @returns {Promise} A promise that resolves when the list comes back from the server.
   * @memberof SentryReleases
   */
  listDeploys(release) {
    return this.execute(['releases', 'deploys', release, 'list'], null);
  }

  /**
   * Creates a new release deployment. This should be called after the release has been
   * finalized, while deploying on a given environment.
   *
   * @example
   * await cli.releases.newDeploy(cli.releases.proposeVersion(), {
   *   // required options:
   *   env: 'production',          // environment for this release. Values that make sense here would be 'production' or 'staging'
   *
   *   // optional options:
   *   started: 42,                // unix timestamp when the deployment started
   *   finished: 1337,             // unix timestamp when the deployment finished
   *   time: 1295,                 // deployment duration in seconds. This can be specified alternatively to `started` and `finished`
   *   name: 'PickleRick',         // human readable name for this deployment
   *   url: 'https://example.com', // URL that points to the deployment
   * });
   *
   * @param {string} release Unique name of the release.
   * @param {object} options Options to configure the new release deploy.
   * @returns {Promise} A promise that resolves when the deploy has been created.
   * @memberof SentryReleases
   */
  newDeploy(release, options) {
    if (!options || !options.env) {
      throw new Error('options.env must be a vaild name');
    }
    const args = ['releases', 'deploys', release, 'new'];
    return this.execute(helper.prepareCommand(args, DEPLOYS_SCHEMA, options), null);
  }

  /**
   * See {helper.execute} docs.
   * @param {string[]} args Command line arguments passed to `sentry-cli`.
   * @param {boolean} live We inherit stdio to display `sentry-cli` output directly.
   * @returns {Promise.<string>} A promise that resolves to the standard output.
   */
  execute(args, live) {
    return helper.execute(args, live, this.options.silent, this.configFile, this.options);
  }
}

module.exports = Releases;
