'use strict';
// @ts-check
const base = require('./.base-ammanrc.js');
const validator = {
    ...base.validator,
    programs: [base.programs.staking],
};
module.exports = {validator};
