// @ts-check
const path = require('path');
const programDir = path.join(__dirname, '../program/staking/');
const idlDir = path.join(__dirname, 'idl');
const sdkDir = path.join(__dirname, 'src', 'generated');
const binaryInstallDir = path.join(__dirname, '.crates');

module.exports = {
  idlGenerator: 'anchor',
  programName: 'staking',
  programId: '2beVdAd5fpgyxwspZBfJGaqTLe2sZBm1KkBxiZFc1Mjr',
  idlDir,
  sdkDir,
  binaryInstallDir,
  programDir,
};
