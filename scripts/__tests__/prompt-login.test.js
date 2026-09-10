const { promptLogin } = require('../prompt-login');

describe('promptLogin', () => {
  test('runs login for an interactive install', () => {
    const spawnSync = jest.fn();

    promptLogin('/path/to/sentry-cli', { isTTY: true }, { isTTY: true }, spawnSync);

    expect(spawnSync).toHaveBeenCalledWith(
      '/path/to/sentry-cli',
      ['login', '--global', '--if-needed'],
      { stdio: 'inherit' }
    );
  });

  test.each([
    [{ isTTY: false }, { isTTY: true }],
    [{ isTTY: true }, { isTTY: false }],
  ])('skips login for a non-interactive install', (stdin, stdout) => {
    const spawnSync = jest.fn();

    promptLogin('/path/to/sentry-cli', stdin, stdout, spawnSync);

    expect(spawnSync).not.toHaveBeenCalled();
  });
});
