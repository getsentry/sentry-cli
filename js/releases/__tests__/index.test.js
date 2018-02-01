/* eslint-env jest */

const SentryCli = require('../../');

describe('SentryCli releases', () => {
  test('call sentry-cli releases propose-version', () => {
    expect.assertions(1);
    const cli = new SentryCli();
    return cli.releases.proposeVersion().then(version => expect(version).toBeTruthy());
  });

  test('call sentry-cli releases upload-sourcemaps', () => {
    expect.assertions(1);
    const cli = new SentryCli();
    return cli.releases
      .uploadSourceMaps('#abc', { include: ['hello'] })
      .then(version => expect(version).toBeTruthy());
  });
});
