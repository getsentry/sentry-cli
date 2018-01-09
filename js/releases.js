'use strict';

/* global Promise */

var helper = require('./helper');

var DEFAULT_IGNORE = ['node_modules'];
var SOURCEMAPS_OPTIONS = {
  ignore: {
    param: '--ignore',
    type: 'array'
  },
  ignoreFile: {
    param: '--ignore-file',
    type: 'string'
  },
  rewrite: {
    param: '--rewrite',
    type: 'boolean'
  },
  sourceMapReference: {
    param: '--no-sourcemap-reference',
    type: 'inverted-boolean'
  },
  stripPrefix: {
    param: '--strip-prefix',
    type: 'array'
  },
  stripCommonPrefix: {
    param: '--strip-common-prefix',
    type: 'array'
  },
  validate: {
    param: '--validate',
    type: 'boolean'
  },
  urlPrefix: {
    param: '--url-prefix',
    type: 'string'
  },
  ext: {
    param: '--ext',
    type: 'string'
  }
};

module.exports = {
  new: function(release) {
    return helper.execute(['releases', 'new', release]);
  },
  finalize: function(release) {
    return helper.execute(['releases', 'finalize', release]);
  },
  proposeVersion: function() {
    return helper.execute(['releases', 'propose-version']);
  },
  uploadSourceMaps: function(options) {
    return Promise.all(
      options.include.map(function(sourcemapPath) {
        var command = [
          'releases',
          'files',
          options.release,
          'upload-sourcemaps',
          sourcemapPath
        ];
        var newOptions = Object.assign({}, options);

        if (!newOptions.ignoreFile && !newOptions.ignore) {
          newOptions.ignore = DEFAULT_IGNORE;
        }

        return helper.execute(
          helper.prepareCommand(command, SOURCEMAPS_OPTIONS, options)
        );
      }, this)
    );
  }
};
