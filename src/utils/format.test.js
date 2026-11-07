// Run with: npm test  (uses Node's built-in test runner, no extra dependencies)
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
    usageLevel,
    temperatureLevel,
    clampPercent,
    formatSpeed,
    formatSpeedCompact,
    formatUptime,
    driveLetter,
    compareVersions,
} from './format.js';

test('usageLevel thresholds', () => {
    assert.equal(usageLevel(null), '');
    assert.equal(usageLevel(74.9), '');
    assert.equal(usageLevel(75), 'warning');
    assert.equal(usageLevel(90), 'danger');
});

test('temperatureLevel thresholds', () => {
    assert.equal(temperatureLevel(undefined), '');
    assert.equal(temperatureLevel(60), '');
    assert.equal(temperatureLevel(80), 'warning');
    assert.equal(temperatureLevel(95), 'danger');
});

test('clampPercent', () => {
    assert.equal(clampPercent(null), 0);
    assert.equal(clampPercent('abc'), 0);
    assert.equal(clampPercent(-5), 0);
    assert.equal(clampPercent(140), 100);
    assert.equal(clampPercent(42.5), 42.5);
});

test('formatSpeed', () => {
    assert.equal(formatSpeed(null), '—');
    assert.equal(formatSpeed(512), '512.0 KB/s');
    assert.equal(formatSpeed(1536), '1.5 MB/s');
    assert.equal(formatSpeed(2 * 1024 * 1024), '2.0 GB/s');
});

test('formatSpeedCompact', () => {
    assert.equal(formatSpeedCompact(0), '0K/s');
    assert.equal(formatSpeedCompact(3.42), '3.4K/s');
    assert.equal(formatSpeedCompact(450.4), '450K/s');
    assert.equal(formatSpeedCompact(1229), '1.2M/s');
    assert.equal(formatSpeedCompact(1.5 * 1024 * 1024), '1.5G/s');
});

test('formatUptime', () => {
    assert.equal(formatUptime(null), '—');
    assert.equal(formatUptime(7 * 60 + 5), '7분');
    assert.equal(formatUptime(5 * 3600 + 12 * 60), '5시간 12분');
    assert.equal(formatUptime(3 * 86400 + 4 * 3600 + 59), '3일 4시간');
});

test('driveLetter', () => {
    assert.equal(driveLetter('C:\\ (Windows)'), 'C:');
    assert.equal(driveLetter('d:\\'), 'd:');
    assert.equal(driveLetter('Data'), 'Da');
    assert.equal(driveLetter(''), '');
});

test('compareVersions', () => {
    assert.equal(compareVersions('0.9.4', '0.9.3'), 1);
    assert.equal(compareVersions('v0.9.3', '0.9.3'), 0);
    assert.equal(compareVersions('0.9.10', '0.9.9'), 1);
    assert.equal(compareVersions('0.10.0', '0.9.9'), 1);
    assert.equal(compareVersions('1.0', '1.0.1'), -1);
    assert.equal(compareVersions('1.0.0-beta.1', '1.0.0'), -1);
    assert.equal(compareVersions('1.0.0', '1.0.0-rc.1'), 1);
});
