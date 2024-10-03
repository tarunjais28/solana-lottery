// @ts-check
'use strict';
const path = require('path');

const localDeployDir = path.join(__dirname, '../../target', 'deploy');
const { LOCALHOST, tmpLedgerDir } = require('@metaplex-foundation/amman');

function localDeployPath(programName) {
    return path.join(localDeployDir, `${programName}.so`);
}

const programs = {
    staking: {
        label: "Staking",
        programId: '2beVdAd5fpgyxwspZBfJGaqTLe2sZBm1KkBxiZFc1Mjr',
        deployPath: localDeployPath('staking'),
    },
};

const validator = {
    killRunningValidators: true,
    programs,
    commitment: 'confirmed',
    resetLedger: true,
    verifyFees: false,
    jsonRpcUrl: LOCALHOST,
    websocketUrl: '',
    ledgerDir: tmpLedgerDir(),
};

module.exports = {
    programs,
    validator,
    relay: {
        enabled: true,
        killRunningRelay: true,
    },
};
