import { OptionsSchema } from '../../helper';

/**
 * Schema for the `sourcemaps inject` command.
 */
export const INJECT_OPTIONS = {
  ignore: {
    param: '--ignore',
    type: 'array',
  },
  ignoreFile: {
    param: '--ignore-file',
    type: 'string',
  },
  ext: {
    param: '--ext',
    type: 'array',
  },
  dryRun: {
    param: '--dry-run',
    type: 'boolean',
  },
} satisfies OptionsSchema;
