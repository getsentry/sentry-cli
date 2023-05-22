const os = require('os');
const path = require('path');

const helper = require('../helper');

const SOURCEMAPS_OPTIONS = require('../releases/options/uploadSourcemaps');

describe('SentryCli helper', () => {
  beforeEach(() => {
    helper.mockBinaryPath(path.resolve(__dirname, '../__mocks__/sentry-cli'));
  });

  test('call sentry-cli --version', () => {
    expect.assertions(1);
    return helper
      .execute(['--version'])
      .then((version) => expect(version.trim()).toBe('sentry-cli DEV'));
  });

  test('call sentry-cli with wrong command', () => {
    expect.assertions(1);
    return helper.execute(['fail']).catch((e) => expect(e.message).toMatch('Command failed:'));
  });

  test('getPath returns platform-appropriate path', () => {
    const pattern = os.platform() === 'win32' ? /sentry-cli.exe$/ : /sentry-cli$/;
    expect(helper.getPath()).toMatch(pattern);
  });

  describe('`prepare` command', () => {
    test('call prepare command add default ignore', () => {
      const command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];

      expect(helper.prepareCommand(command)).toEqual([
        'releases',
        'files',
        'release',
        'upload-sourcemaps',
        '/dev/null',
      ]);
    });

    test('call prepare command with array option', () => {
      const command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { stripPrefix: ['node', 'app'] })
      ).toEqual([
        'releases',
        'files',
        'release',
        'upload-sourcemaps',
        '/dev/null',
        '--strip-prefix',
        'node',
        '--strip-prefix',
        'app',
      ]);

      // Should throw since it is no array
      expect(() => {
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { stripPrefix: 'node' });
      }).toThrow();
    });

    test('call prepare command with boolean option', () => {
      const command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { sourceMapReference: false })
      ).toEqual([
        'releases',
        'files',
        'release',
        'upload-sourcemaps',
        '/dev/null',
        '--no-sourcemap-reference',
      ]);

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { sourceMapReference: true })
      ).toEqual(['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null']);

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { decompress: true, rewrite: true })
      ).toEqual([
        'releases',
        'files',
        'release',
        'upload-sourcemaps',
        '/dev/null',
        '--decompress',
        '--rewrite',
      ]);

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { rewrite: false, dedupe: false })
      ).toEqual([
        'releases',
        'files',
        'release',
        'upload-sourcemaps',
        '/dev/null',
        '--no-rewrite',
        '--no-dedupe',
      ]);

      // Only `invertedParam` registered for `dedupe` in `uploadSourcemaps`, so it should not add anything for positive boolean.
      expect(helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { dedupe: true })).toEqual([
        'releases',
        'files',
        'release',
        'upload-sourcemaps',
        '/dev/null',
      ]);

      expect(() => {
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { sourceMapReference: 'node' });
      }).toThrow();
    });

    test('call prepare command with string option', () => {
      const command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];

      expect(helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { ext: ['js'] })).toEqual([
        'releases',
        'files',
        'release',
        'upload-sourcemaps',
        '/dev/null',
        '--ext',
        'js',
      ]);

      expect(helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { urlPrefix: '~/' })).toEqual([
        'releases',
        'files',
        'release',
        'upload-sourcemaps',
        '/dev/null',
        '--url-prefix',
        '~/',
      ]);

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { urlSuffix: '?hash=1337' })
      ).toEqual([
        'releases',
        'files',
        'release',
        'upload-sourcemaps',
        '/dev/null',
        '--url-suffix',
        '?hash=1337',
      ]);

      expect(helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { decompress: true })).toEqual([
        'releases',
        'files',
        'release',
        'upload-sourcemaps',
        '/dev/null',
        '--decompress',
      ]);

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { ignoreFile: '/js.ignore' })
      ).toEqual([
        'releases',
        'files',
        'release',
        'upload-sourcemaps',
        '/dev/null',
        '--ignore-file',
        '/js.ignore',
      ]);
    });
  });
});
