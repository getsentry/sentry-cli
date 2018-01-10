'use strict';

/* global Promise */

var helper = require('../helper');

var DEFAULT_IGNORE = ['node_modules'];
var SOURCEMAPS_OPTIONS = require('./options/uploadSourcemaps');

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
  uploadSourceMaps: function(release, options) {
    if (typeof options === 'undefined' || typeof options.include === 'undefined') {
      throw new Error('options.include must be a vaild path(s)');
    }
    return Promise.all(
      options.include.map(function(sourcemapPath) {
        var command = ['releases', 'files', release, 'upload-sourcemaps', sourcemapPath];
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
