/* eslint-env jest */

var SentryCli = require('../../');

describe('SentryCli releases', function() {
  test('call sentry-cli releases propose-version', function() {
    expect.assertions(1);
    var cli = new SentryCli();
    return cli.releases.proposeVersion().then(function(version) {
      expect(version).toBeTruthy();
    });
  });
  test('call sentry-cli releases upload-sourcemaps', function() {
    expect.assertions(1);
    var cli = new SentryCli();
    return cli.releases
      .uploadSourceMaps('#abc', { include: ['hello'] })
      .then(function(version) {
        expect(version).toBeTruthy();
      });
  });
});
