const os = require('os');

const helper = require('../helper');

const SOURCEMAPS_OPTIONS = require('../releases/options/uploadSourcemaps');

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

  describe('silentLogs functionality', () => {
    let consoleInfoSpy;
    let consoleErrorSpy;

    beforeEach(() => {
      consoleInfoSpy = jest.spyOn(console, 'info').mockImplementation();
      consoleErrorSpy = jest.spyOn(console, 'error').mockImplementation();
    });

    afterEach(() => {
      consoleInfoSpy.mockRestore();
      consoleErrorSpy.mockRestore();
    });

    test('determineSuccessMessage returns correct message for releases new', () => {
      const args = ['releases', 'new', 'v1.0.0'];
      const message = helper.determineSuccessMessage(args);
      expect(message).toBe('✓ Release v1.0.0 created');
    });

    test('determineSuccessMessage returns correct message for sourcemaps upload', () => {
      const args = ['sourcemaps', 'upload'];
      const message = helper.determineSuccessMessage(args);
      expect(message).toBe('✓ Source maps uploaded');
    });

    test('determineSuccessMessage returns null for version check', () => {
      const args = ['--version'];
      const message = helper.determineSuccessMessage(args);
      expect(message).toBe(null);
    });

    test('determineSuccessMessage handles empty args', () => {
      const message = helper.determineSuccessMessage([]);
      expect(message).toBe(null);
    });

    test('determineSuccessMessage handles null args', () => {
      const message = helper.determineSuccessMessage(null);
      expect(message).toBe(null);
    });

    test('execute with silent=true takes precedence over silentLogs=true', async () => {
      const result = await helper.execute(['--version'], false, true, true);

      expect(result).toBe('');
      expect(consoleInfoSpy).not.toHaveBeenCalled();
    });
  });

  describe('command type coverage', () => {
    test('determineSuccessMessage handles supported high-impact commands', () => {
      const testCases = [
        { args: ['releases', 'new', 'v1.0.0'], expected: '✓ Release v1.0.0 created' },
        { args: ['releases', 'finalize', 'v1.0.0'], expected: '✓ Release v1.0.0 finalized' },
        {
          args: ['releases', 'files', 'v1.0.0', 'upload-sourcemaps'],
          expected: '✓ Source maps uploaded to release v1.0.0',
        },
        { args: ['sourcemaps', 'upload'], expected: '✓ Source maps uploaded' },
        { args: ['sourcemaps', 'inject'], expected: '✓ Source maps injected' },
        { args: ['debug-files', 'upload'], expected: '✓ Debug files uploaded' },
        { args: ['upload-proguard'], expected: '✓ ProGuard mappings uploaded' },
        { args: ['upload-dif'], expected: '✓ Debug information files uploaded' },
        { args: ['upload-dsym'], expected: '✓ dSYM files uploaded' },
        { args: ['deploys', 'new'], expected: '✓ Deploy created' },
        { args: ['mobile-app', 'upload'], expected: '✓ Mobile app uploaded' },
        { args: ['send-event'], expected: '✓ Event sent' },
        { args: ['send-envelope'], expected: '✓ Envelope sent' },
        { args: ['send-metric'], expected: '✓ Metric sent' },
      ];

      testCases.forEach(({ args, expected }) => {
        const message = helper.determineSuccessMessage(args);
        expect(message).toBe(expected);
      });
    });

    test('determineSuccessMessage returns null for info/list operations and utility commands', () => {
      const testCases = [
        ['--help'],
        ['--version'],
        ['info'],
        ['login'],
        ['organizations', 'list'],
        ['projects', 'list'],
        ['issues', 'list'],
        ['events', 'list'],
        ['files', 'list'],
        ['deploys', 'list'],
        ['monitors', 'list'],
        ['releases', 'list'],
        ['releases', 'delete', 'v1.0.0'],
        ['unknown-command'],
      ];

      testCases.forEach((args) => {
        const message = helper.determineSuccessMessage(args);
        expect(message).toBe(null);
      });
    });
  });

  describe('Promise resolution scenarios', () => {
    let consoleInfoSpy;

    beforeEach(() => {
      consoleInfoSpy = jest.spyOn(console, 'info').mockImplementation();
    });

    afterEach(() => {
      consoleInfoSpy.mockRestore();
    });

    test('execute with live=true, silentLogs=true uses stdio piping and resolves with undefined', async () => {
      const result = await helper.execute(['--version'], true, false, true);

      expect(result).toBeUndefined();
      expect(consoleInfoSpy).not.toHaveBeenCalled(); // --version doesn't have success message
    });

    test('execute with live=true, silentLogs=true shows success message for supported commands', async () => {
      const result = await helper.execute(['sourcemaps', 'upload'], true, false, true);

      expect(result).toBeUndefined();
      expect(consoleInfoSpy).toHaveBeenCalledWith('✓ Source maps uploaded');
    });

    test('execute with live=false, silentLogs=true uses callback mode and resolves with empty string', async () => {
      const result = await helper.execute(['--version'], false, false, true);

      expect(result).toBe('');
      expect(consoleInfoSpy).not.toHaveBeenCalled();
    });

    test('execute with live=false, silentLogs=true shows success message and returns empty string', async () => {
      const result = await helper.execute(['sourcemaps', 'upload'], false, false, true);

      expect(result).toBe('');
      expect(consoleInfoSpy).toHaveBeenCalledWith('✓ Source maps uploaded');
    });

    test('execute with normal mode (live=false, silent=false, silentLogs=false) returns actual stdout', async () => {
      const result = await helper.execute(['--version'], false, false, false);

      expect(result.trim()).toBe('sentry-cli DEV');
      expect(consoleInfoSpy).not.toHaveBeenCalled();
    });

    test('execute with silent=true suppresses output and messages', async () => {
      // Test with live=true - uses stdio piping, resolves with undefined
      const result1 = await helper.execute(['--version'], true, true, false);
      expect(result1).toBeUndefined();

      // Test with live=false - uses callback mode, resolves with empty string
      const result2 = await helper.execute(['--version'], false, true, false);
      expect(result2).toBe('');

      // Test with silentLogs=true (should be ignored when silent=true)
      const result3 = await helper.execute(['sourcemaps', 'upload'], false, true, true);
      expect(result3).toBe('');

      // No success messages should be shown when silent=true
      expect(consoleInfoSpy).not.toHaveBeenCalled();
    });

    test('execute with live=true and normal mode uses stdio inherit and resolves with undefined', async () => {
      const result = await helper.execute(['--version'], true, false, false);

      // Should resolve with undefined (stdio inherit mode)
      expect(result).toBeUndefined();
      expect(consoleInfoSpy).not.toHaveBeenCalled();
    });
  });
});
