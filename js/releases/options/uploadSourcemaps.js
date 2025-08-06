/**
 * @type {import('../../helper').OptionsSchema}
 */
module.exports = {
  ignore: {
    param: '--ignore',
    type: 'array',
  },
  ignoreFile: {
    param: '--ignore-file',
    type: 'string',
  },
  dist: {
    param: '--dist',
    type: 'string',
  },
  decompress: {
    param: '--decompress',
    type: 'boolean',
  },
  rewrite: {
    param: '--rewrite',
    invertedParam: '--no-rewrite',
    type: 'boolean',
  },
  sourceMapReference: {
    invertedParam: '--no-sourcemap-reference',
    type: 'boolean',
  },
  dedupe: {
    invertedParam: '--no-dedupe',
    type: 'boolean',
  },
  stripPrefix: {
    param: '--strip-prefix',
    type: 'array',
  },
  stripCommonPrefix: {
    param: '--strip-common-prefix',
    type: 'boolean',
  },
  validate: {
    param: '--validate',
    type: 'boolean',
  },
  urlPrefix: {
    param: '--url-prefix',
    type: 'string',
  },
  urlSuffix: {
    param: '--url-suffix',
    type: 'string',
  },
  ext: {
    param: '--ext',
    type: 'array',
  },
  useArtifactBundle: {
    // Deprecated option - no param to avoid passing --use-artifact-bundle to CLI
    // param: '--use-artifact-bundle', // REMOVED - this flag is deprecated in CLI
    type: 'boolean',
    deprecated: true, // Custom flag to identify deprecated options
  },
};
