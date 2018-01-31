'use strict';

const helper = require('../helper');

const DEFAULT_IGNORE = ['node_modules'];
const SOURCEMAPS_OPTIONS = require('./options/uploadSourcemaps');

module.exports = {
  new(release) {
    return helper.execute(['releases', 'new', release]);
  },

  finalize(release) {
    return helper.execute(['releases', 'finalize', release]);
  },

  proposeVersion() {
    return helper.execute(['releases', 'propose-version']);
  },

  uploadSourceMaps(release, options) {
    if (!options || !options.include) {
      throw new Error('options.include must be a vaild path(s)');
    }

    return Promise.all(
      options.include.map(sourcemapPath => {
        const args = ['releases', 'files', release, 'upload-sourcemaps', sourcemapPath];
        const newOptions = Object.assign({}, options);

        if (!newOptions.ignoreFile && !newOptions.ignore) {
          newOptions.ignore = DEFAULT_IGNORE;
        }

        return helper.execute(helper.prepareCommand(args, SOURCEMAPS_OPTIONS, options));
      })
    );
  },
};
