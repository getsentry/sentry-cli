import { OptionsSchema } from '../../helper';

/**
 * Schema for the `debug-files prepare` command.
 */
export const PREPARE_OPTIONS = {
  ignore: {
    param: '--ignore',
    type: 'array',
  },
  ignoreFile: {
    param: '--ignore-file',
    type: 'string',
  },
  outDir: {
    param: '--out-dir',
    type: 'string',
  },
  stripNames: {
    param: '--strip-names',
    type: 'boolean',
  },
  upload: {
    invertedParam: '--no-upload',
    type: 'boolean',
  },
  dryRun: {
    param: '--dry-run',
    type: 'boolean',
  },
  requireDwarf: {
    param: '--require-dwarf',
    type: 'boolean',
  },
  includeSources: {
    param: '--include-sources',
    type: 'boolean',
  },
  wait: {
    param: '--wait',
    type: 'boolean',
  },
  waitFor: {
    param: '--wait-for',
    type: 'number',
  },
} satisfies OptionsSchema;
