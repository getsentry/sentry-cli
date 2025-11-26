import { OptionsSchema } from '../../helper';

/**
 * Schema for the `deploys new` command.
 */
export const DEPLOYS_OPTIONS = {
  env: {
    param: '--env',
    type: 'string',
  },
  started: {
    param: '--started',
    type: 'number',
  },
  finished: {
    param: '--finished',
    type: 'number',
  },
  time: {
    param: '--time',
    type: 'number',
  },
  name: {
    param: '--name',
    type: 'string',
  },
  url: {
    param: '--url',
    type: 'string',
  },
} satisfies OptionsSchema;
