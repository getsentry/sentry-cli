'use strict';

import { SentryCliInjectOptions, SentryCliOptions } from '../types';
import { INJECT_OPTIONS } from './options/inject';
import * as helper from '../helper';

/**
 * Default arguments for the `--ignore` option.
 */
const DEFAULT_IGNORE: string[] = ['node_modules'];

/**
 * Manages source map operations on Sentry.
 */
export class SourceMaps {
  constructor(
    public options: SentryCliOptions = {},
    private configFile: string | null
  ) {}

  /**
   * Fixes up JavaScript source files and source maps with debug ids.
   *
   * For every minified JS source file, a debug id is generated and
   * inserted into the file. If the source file references a
   * source map and that source map is locally available,
   * the debug id will be injected into it as well.
   * If the referenced source map already contains a debug id,
   * that id is used instead.
   *
   * @example
   * await cli.sourceMaps.inject({
   *   // required options:
   *   paths: ['./dist'],
   *
   *   // default options:
   *   ignore: ['node_modules'],  // globs for files to ignore
   *   ignoreFile: null,          // path to a file with ignore rules
   *   ext: ['js', 'cjs', 'mjs'], // file extensions to consider
   *   dryRun: false,             // don't modify files on disk
   * });
   *
   * @param options Options to configure the debug id injection.
   * @returns A promise that resolves when the injection has completed successfully.
   */
  async inject(options: SentryCliInjectOptions): Promise<string> {
    if (!options || !options.paths || !Array.isArray(options.paths)) {
      throw new Error('`options.paths` must be a valid array of paths.');
    }

    if (options.paths.length === 0) {
      throw new Error('`options.paths` must contain at least one path.');
    }

    const newOptions = { ...options };
    if (!newOptions.ignoreFile && !newOptions.ignore) {
      newOptions.ignore = DEFAULT_IGNORE;
    }

    const args = helper.prepareCommand(
      ['sourcemaps', 'inject', ...options.paths],
      INJECT_OPTIONS,
      newOptions
    );

    return this.execute(args, true);
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
