module.exports = {
  ignore: {
    param: '--ignore',
    type: 'array',
  },
  ignoreFile: {
    param: '--ignore-file',
    type: 'string',
  },
  rewrite: {
    param: '--rewrite',
    invertedParam: '--no-rewrite',
    type: 'toggle',
  },
  sourceMapReference: {
    param: '--no-sourcemap-reference',
    type: 'inverted-boolean',
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
};
