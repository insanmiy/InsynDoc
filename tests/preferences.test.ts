import { describe, expect, it } from 'vitest';
import { DEFAULT_PREFERENCES, parsePreferences } from '../src/preferences';

describe('Appearance Preferences', () => {
  it('Validates And Restores Learned Corrections', () => {
    expect(parsePreferences(JSON.stringify({ spellingCorrections: [
      { source: 'Teh', replacement: 'tech', count: 3 },
      { source: 'teh', replacement: 'the', count: -1 },
      { source: 'bad', replacement: '<script>', count: 1 },
      null
    ] }))).toMatchObject({ spellingCorrections: [{ source: 'teh', replacement: 'tech', count: 3 }] });
    expect(parsePreferences(null)).toMatchObject({ spellingCorrections: [] });
  });
  it('Enables Spelling By Default And Validates The Personal Dictionary', () => {
    expect(parsePreferences(null).checkSpelling).toBe(true);
    expect(parsePreferences('{"checkSpelling":false,"spellingWords":["InsynDoc","InsynDoc",42,"", "two words"]}'))
      .toMatchObject({ checkSpelling: false, spellingWords: ['InsynDoc'] });
    expect(parsePreferences('{"checkSpelling":"false","spellingWords":{}}'))
      .toMatchObject({ checkSpelling: true, spellingWords: [] });
  });
  it('Uses The System Browser Until A Custom Browser Is Chosen', () => {
    for (const browserPath of [null, false, 42, '', '   ']) {
      expect(parsePreferences(JSON.stringify({ browserPath })).browserPath).toBeNull();
    }
    const browserPath = 'C:/Program Files/Firefox/firefox.exe';
    expect(parsePreferences(JSON.stringify({ browserPath })).browserPath).toBe(browserPath);
  });
  it('Uses Defaults For Missing Or Damaged Settings', () => {
    for (const value of [null, '', '{', 'null', 'false', '[]']) {
      expect(parsePreferences(value)).toEqual(DEFAULT_PREFERENCES);
    }
  });

  it('Restores Valid Settings Without Trusting Unknown Values', () => {
    expect(parsePreferences('{"theme":"dark","showRuler":false,"showOutline":false,"showWordCount":false}'))
      .toEqual({ ...DEFAULT_PREFERENCES, theme: 'dark', showRuler: false, showOutline: false, showWordCount: false });
    expect(parsePreferences('{"theme":"green","showRuler":"false"}')).toEqual(DEFAULT_PREFERENCES);
  });

  it('Restores Ruler Units And Validates Writing Goals', () => {
    expect(parsePreferences('{"rulerUnit":"cm","wordGoal":1500}')).toMatchObject({ rulerUnit: 'cm', wordGoal: 1500 });
    for (const wordGoal of [-1, 1.5, 100001, '500']) {
      expect(parsePreferences(JSON.stringify({ wordGoal, rulerUnit: 'meters' }))).toMatchObject({ wordGoal: 0, rulerUnit: 'inch' });
    }
  });
});
