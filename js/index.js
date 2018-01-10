'use strict';

var pkgInfo = require('../package.json');
var helper = require('./helper');

function SentryCli(configFile) {
  if (typeof configFile === 'string') process.env.SENTRY_PROPERTIES = configFile;
}

SentryCli.prototype.getConfigStatus = function() {
  return helper.execute(['info', '--config-status-json']);
};

SentryCli.getVersion = function() {
  return pkgInfo.version;
};

SentryCli.getPath = function() {
  return helper.getPath();
};

SentryCli.prototype.releases = require('./releases');

module.exports = SentryCli;
