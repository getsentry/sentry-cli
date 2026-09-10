'use strict';

const childProcess = require('child_process');

function promptLogin(
  binaryPath,
  stdin = process.stdin,
  stdout = process.stdout,
  spawnSync = childProcess.spawnSync
) {
  if (!stdin.isTTY || !stdout.isTTY) {
    return;
  }

  spawnSync(binaryPath, ['login', '--global', '--if-needed'], {
    stdio: 'inherit',
  });
}

module.exports = { promptLogin };
