'use strict';

import {
  SentryCliCommitsOptions,
  SentryCliNewDeployOptions,
  SentryCliOptions,
  SentryCliUploadSourceMapsOptions,
} from '../types';
import { DEPLOYS_OPTIONS } from './options/deploys';
import { SOURCEMAPS_OPTIONS } from './options/uploadSourcemaps';
import * as helper from '../helper';

/**
 * Default arguments for the `--ignore` option.
 */
const DEFAULT_IGNORE: string[] = ['node_modules'];

/**
 * Manages releases and release artifacts on Sentry.
 */
export class Releases {
  constructor(public options: SentryCliOptions = {}, private configFile: string | null) {}

  /**
   * Registers a new release with sentry.
   *
   * The given release name should be unique and deterministic. It can later be used to
   * upload artifacts, such as source maps.
   *
   * @param release Unique name of the new release.
   * @param options The list of project slugs for a release.
   * @returns A promise that resolves when the release has been created.
   */
  async new(release: string, options: { projects?: string[] }): Promise<string> {
    const args = ['releases', 'new', release].concat(helper.getProjectFlagsFromOptions(options));
    return this.execute(args, null);
  }

  /**
   * Specifies the set of commits covered in this release.
   *
   * @param release Unique name of the release
   * @param options A set of options to configure the commits to include
   * @returns A promise that resolves when the commits have been associated
   */
  async setCommits(release: string, options: SentryCliCommitsOptions): Promise<string> {
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

    if (options.ignoreMissing) {
      commitFlags.push('--ignore-missing');
    }

    return this.execute(['releases', 'set-commits', release].concat(commitFlags), false);
  }

  /**
   * Marks this release as complete. This should be called once all artifacts has been
   * uploaded.
   *
   * @param release Unique name of the release.
   * @returns A promise that resolves when the release has been finalized.
   */
  async finalize(release: string): Promise<string> {
    return this.execute(['releases', 'finalize', release], null);
  }

  /**
   * Creates a unique, deterministic version identifier based on the project type and
   * source files. This identifier can be used as release name.
   *
   * @returns A promise that resolves to the version string.
   */
  async proposeVersion(): Promise<string> {
    const version = await this.execute(['releases', 'propose-version'], null);
    return version.trim();
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
   *   dedupe: true,              // deduplicate already uploaded files
   *   stripPrefix: [],           // remove certain prefixes from filenames
   *   stripCommonPrefix: false,  // guess common prefixes to remove from filenames
   *   validate: false,           // validate source maps and cancel the upload on error
   *   urlPrefix: '',             // add a prefix source map urls after stripping them
   *   urlSuffix: '',             // add a suffix source map urls after stripping them
   *   ext: ['js', 'map', 'jsbundle', 'bundle'],  // override file extensions to scan for
   *   projects: ['node'],        // provide a list of projects
   *   decompress: false          // decompress gzip files before uploading
   * });
   *
   * @param release Unique name of the release.
   * @param options Options to configure the source map upload.
   * @returns A promise that resolves when the upload has completed successfully.
   */
  async uploadSourceMaps(
    release: string,
    options: SentryCliUploadSourceMapsOptions
  ): Promise<string[]> {
    if (!options || !options.include || !Array.isArray(options.include)) {
      throw new Error(
        '`options.include` must be a valid array of paths and/or path descriptor objects.'
      );
    }

    // Each entry in the `include` array will map to an array of promises, which
    // will in turn contain one promise per literal path value. Thus `uploads`
    // will be an array of Promise arrays, which we'll flatten later.
    const uploads = options.include.map((includeEntry) => {
      let pathOptions;
      let uploadPaths;

      if (typeof includeEntry === 'object') {
        pathOptions = includeEntry;
        uploadPaths = includeEntry.paths;

        if (!Array.isArray(uploadPaths)) {
          throw new Error(
            `Path descriptor objects in \`options.include\` must contain a \`paths\` array. Got ${includeEntry}.`
          );
        }
      }
      // `includeEntry` should be a string, which we can wrap in an array to
      // match the `paths` property in the descriptor object type
      else {
        pathOptions = {};
        uploadPaths = [includeEntry];
      }

      const newOptions = { ...options, ...pathOptions };
      if (!newOptions.ignoreFile && !newOptions.ignore) {
        newOptions.ignore = DEFAULT_IGNORE;
      }

      // args which apply to the entire `include` entry (everything besides the path)
      const args = ['sourcemaps', 'upload']
        .concat(helper.getProjectFlagsFromOptions(options))
        .concat(['--release', release]);

      return uploadPaths.map((path) =>
        // `execute()` is async and thus we're returning a promise here
        this.execute(helper.prepareCommand([...args, path], SOURCEMAPS_OPTIONS, newOptions), true)
      );
    });

    // `uploads` is an array of Promise arrays, which needs to be flattened
    // before being passed to `Promise.all()`. (`Array.flat()` doesn't exist in
    // Node < 11; this polyfill takes advantage of the fact that `concat()` is
    // willing to accept an arbitrary number of items to add to and/or iterables
    // to unpack into the given array.)
    return Promise.all([].concat(...uploads));
  }

  /**
   * List all deploys for a given release.
   *
   * @param release Unique name of the release.
   * @returns A promise that resolves when the list comes back from the server.
   */
  async listDeploys(release: string): Promise<string> {
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
   *   projects: ['project1', 'project2'], // list of projects to deploy to
   * });
   *
   * @param release Unique name of the release.
   * @param options Options to configure the new release deploy.
   * @returns A promise that resolves when the deploy has been created.
   */
  async newDeploy(release: string, options: SentryCliNewDeployOptions): Promise<string> {
    if (!options || !options.env) {
      throw new Error('options.env must be a valid name');
    }
    const args = ['releases', 'deploys']
      .concat(helper.getProjectFlagsFromOptions(options))
      .concat([release, 'new']);
    return this.execute(helper.prepareCommand(args, DEPLOYS_OPTIONS, options), null);
  }

  /**
   * See {helper.execute} docs.
   * @param args Command line arguments passed to `sentry-cli`.
   * @param live can be set to:
   *  - `true` to inherit stdio and reject the promise if the command
   *    exits with a non-zero exit code.
   *  - `false` to not inherit stdio and return the output as a string.
   * @returns A promise that resolves to the standard output.
   */
  async execute(args: string[], live: boolean): Promise<string> {
    return helper.execute(args, live, this.options.silent, this.configFile, this.options);
  }
}
