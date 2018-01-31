'use strict';

const pkgInfo = require('../package.json');
const helper = require('./helper');

class SentryCli {
  constructor(configFile) {
    if (typeof configFile === 'string') {
      process.env.SENTRY_PROPERTIES = configFile;
    }
  }

  static getVersion() {
    return pkgInfo.version;
  }

  static getPath() {
    return helper.getPath();
  }
}

SentryCli.prototype.releases = require('./releases');

module.exports = SentryCli;
