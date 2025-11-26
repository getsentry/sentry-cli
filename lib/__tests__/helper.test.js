const os = require('os');

const helper = require('../helper');

const { SOURCEMAPS_OPTIONS } = require('../releases/options/uploadSourcemaps');

describe('SentryCli helper', () => {
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

  describe('execute', () => {
    test('execute with live=false returns stdout', async () => {
      const output = await helper.execute(['--version'], false);
      expect(output.trim()).toBe('sentry-cli DEV');
    });

    test('execute with live=true resolves on success', async () => {
      // TODO (v3): This should resolve with a string, not undefined/void
      const result = await helper.execute(['--version'], true);
      expect(result).toBe('success (live mode)');
    });

    test('execute with live=true rejects on failure', async () => {
      await expect(helper.execute(['fail'], true)).rejects.toThrow(
        'Command fail failed with exit code 1'
      );
    });

    // live=rejectOnError is not supported per the type declarations, but we should still aim
    // to support it for backwards compatibility.
    test('execute with live=rejectOnError resolves on success', async () => {
      const result = await helper.execute(['--version'], 'rejectOnError');
      expect(result).toBe('success (live mode)');
    });

    // live=rejectOnError is not supported per the type declarations, but we should still aim
    // to support it for backwards compatibility.
    test('execute with live=rejectOnError rejects on failure', async () => {
      await expect(helper.execute(['fail'], 'rejectOnError')).rejects.toThrow(
        'Command fail failed with exit code 1'
      );
    });
  });

  describe('`prepare` command', () => {
    test('call prepare command add default ignore', () => {
      const command = ['sourcemaps', 'upload', '--release', 'release', '/dev/null'];

      expect(helper.prepareCommand(command)).toEqual([
        'sourcemaps',
        'upload',
        '--release',
        'release',
        '/dev/null',
      ]);
    });

    test('call prepare command with array option', () => {
      const command = ['sourcemaps', 'upload', '--release', 'release', '/dev/null'];

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { stripPrefix: ['node', 'app'] })
      ).toEqual([
        'sourcemaps',
        'upload',
        '--release',
        'release',
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
      const command = ['sourcemaps', 'upload', '--release', 'release', '/dev/null'];

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { sourceMapReference: false })
      ).toEqual([
        'sourcemaps',
        'upload',
        '--release',
        'release',
        '/dev/null',
        '--no-sourcemap-reference',
      ]);

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { sourceMapReference: true })
      ).toEqual(['sourcemaps', 'upload', '--release', 'release', '/dev/null']);

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { decompress: true, rewrite: true })
      ).toEqual([
        'sourcemaps',
        'upload',
        '--release',
        'release',
        '/dev/null',
        '--decompress',
        '--rewrite',
      ]);

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { rewrite: false, dedupe: false })
      ).toEqual([
        'sourcemaps',
        'upload',
        '--release',
        'release',
        '/dev/null',
        '--no-rewrite',
        '--no-dedupe',
      ]);

      // Only `invertedParam` registered for `dedupe` in `uploadSourcemaps`, so it should not add anything for positive boolean.
      expect(helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { dedupe: true })).toEqual([
        'sourcemaps',
        'upload',
        '--release',
        'release',
        '/dev/null',
      ]);

      expect(() => {
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { sourceMapReference: 'node' });
      }).toThrow();
    });

    test('call prepare command with string option', () => {
      const command = ['sourcemaps', 'upload', '--release', 'release', '/dev/null'];

      expect(helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { ext: ['js'] })).toEqual([
        'sourcemaps',
        'upload',
        '--release',
        'release',
        '/dev/null',
        '--ext',
        'js',
      ]);

      expect(helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { urlPrefix: '~/' })).toEqual([
        'sourcemaps',
        'upload',
        '--release',
        'release',
        '/dev/null',
        '--url-prefix',
        '~/',
      ]);

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { urlSuffix: '?hash=1337' })
      ).toEqual([
        'sourcemaps',
        'upload',
        '--release',
        'release',
        '/dev/null',
        '--url-suffix',
        '?hash=1337',
      ]);

      expect(helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { decompress: true })).toEqual([
        'sourcemaps',
        'upload',
        '--release',
        'release',
        '/dev/null',
        '--decompress',
      ]);

      expect(
        helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { ignoreFile: '/js.ignore' })
      ).toEqual([
        'sourcemaps',
        'upload',
        '--release',
        'release',
        '/dev/null',
        '--ignore-file',
        '/js.ignore',
      ]);
    });
  });
});
