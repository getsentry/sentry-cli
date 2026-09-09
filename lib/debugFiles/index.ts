'use strict';

import { SentryCliDebugFilesPrepareOptions, SentryCliOptions } from '../types';
import { PREPARE_OPTIONS } from './options/prepare';
import * as helper from '../helper';

/**
 * Default arguments for the `--ignore` option.
 */
const DEFAULT_IGNORE: string[] = ['node_modules'];

/**
 * Manages debug information file operations on Sentry.
 */
export class DebugFiles {
  constructor(
    public options: SentryCliOptions = {},
    private configFile: string | null
  ) {}

  /**
   * Split WebAssembly DWARF into `*.debug.wasm` companions and upload them.
   *
   * For every `.wasm` with DWARF, injects a `build_id` if missing, writes a
   * debug companion that keeps the Code section, and strips DWARF from the
   * deployable module. Name/symtab-only modules are skipped with a warning.
   *
   * @example
   * await cli.debugFiles.prepare({
   *   path: './dist',
   *   upload: true,
   *   includeSources: true,
   *   wait: true,
   * });
   *
   * @param options Options to configure prepare and upload.
   * @returns A promise that resolves when prepare (and optional upload) has completed.
   */
  async prepare(options: SentryCliDebugFilesPrepareOptions): Promise<string> {
    const paths = normalizePreparePaths(options);
    if (paths.length === 0) {
      throw new Error('`options.path` or `options.paths` must contain at least one path.');
    }

    const newOptions: Record<string, unknown> = { ...options };
    if (!newOptions.ignoreFile && !newOptions.ignore) {
      newOptions.ignore = DEFAULT_IGNORE;
    }

    const args = helper.prepareCommand(
      ['debug-files', 'prepare', ...paths],
      PREPARE_OPTIONS,
      newOptions
    );

    return this.execute(args, true);
  }

  /**
   * See {helper.execute} docs.
   */
  async execute(args: string[], live: boolean): Promise<string> {
    return helper.execute(args, live, this.options.silent, this.configFile, this.options);
  }
}

function normalizePreparePaths(options: SentryCliDebugFilesPrepareOptions | undefined): string[] {
  if (!options) {
    return [];
  }

  const fromPath = options.path;
  const fromPaths = options.paths;

  const collected: string[] = [];
  if (typeof fromPath === 'string') {
    collected.push(fromPath);
  } else if (Array.isArray(fromPath)) {
    collected.push(...fromPath);
  }

  if (Array.isArray(fromPaths)) {
    collected.push(...fromPaths);
  }

  return collected;
}
