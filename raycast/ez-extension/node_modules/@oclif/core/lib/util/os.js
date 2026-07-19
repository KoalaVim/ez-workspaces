"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.getHomeDir = getHomeDir;
exports.getPlatform = getPlatform;
exports.getShell = getShell;
const is_wsl_1 = __importDefault(require("is-wsl"));
const node_child_process_1 = require("node:child_process");
const node_os_1 = require("node:os");
const node_path_1 = __importDefault(require("node:path"));
/**
 * Call os.homedir() and return the result
 *
 * Wrapping this allows us to stub these in tests since os.homedir() is
 * non-configurable and non-writable.
 *
 * @returns The user's home directory
 */
function getHomeDir() {
    return (0, node_os_1.homedir)();
}
/**
 * Call os.platform() and return the result
 *
 * Wrapping this allows us to stub these in tests since os.platform() is
 * non-configurable and non-writable.
 *
 * @returns The process' platform
 */
function getPlatform() {
    return is_wsl_1.default ? 'wsl' : (0, node_os_1.platform)();
}
function getShell() {
    let shellPath;
    const SHELL = process.env.SHELL ?? (0, node_os_1.userInfo)().shell?.split(node_path_1.default.sep)?.pop();
    if (SHELL) {
        shellPath = SHELL.split('/');
    }
    else if (getPlatform() === 'win32') {
        shellPath = determineWindowsShell().split(node_path_1.default.sep);
    }
    else {
        shellPath = ['unknown'];
    }
    return shellPath.at(-1) ?? 'unknown';
}
function determineWindowsShell() {
    try {
        const parentProcessName = (0, node_child_process_1.execSync)(`powershell.exe -NoProfile -Command "(Get-CimInstance Win32_Process -Filter 'ProcessID = ${process.ppid}').Name"`, { encoding: 'utf8' });
        return parentProcessName.includes('powershell') || parentProcessName.includes('pwsh')
            ? 'powershell'
            : (process.env.COMSPEC ?? 'cmd.exe');
    }
    catch {
        return process.env.COMSPEC ?? 'cmd.exe';
    }
}
