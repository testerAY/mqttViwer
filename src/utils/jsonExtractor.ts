/**
 * Extracts a value from a JSON string or object based on a dot-notation key path.
 * 
 * @param payload The raw string payload or an object
 * @param key The dot-notation key path (e.g. "sensor.temp")
 * @returns The extracted value, or undefined if not found/parse error
 */
export function extractValue(payload: string | any, key?: string): any {
  // If no key is provided, return the payload directly
  if (!key || key.trim() === '') {
    return payload;
  }

  let data = payload;

  // Try to parse string payload as JSON
  if (typeof payload === 'string') {
    try {
      // Simple check to avoid parsing simple strings or numbers unnecessarily
      // though JSON.parse handles "123" correctly.
      data = JSON.parse(payload);
    } catch (e) {
      // Try to recover from common JSON formatting issues like single quotes
      // e.g. {'x': 1} -> {"x": 1}
      try {
        const corrected = payload.replace(/'/g, '"');
        data = JSON.parse(corrected);
      } catch (e2) {
        // Still invalid, cannot extract with key
        return undefined;
      }
    }
  }

  if (data === null || typeof data !== 'object') {
    return undefined;
  }

  // Navigate the object using the key path
  const keys = key.split('.');
  let current = data;

  for (const k of keys) {
    if (current === null || current === undefined) {
      return undefined;
    }
    // Check if it's an object/array before trying to access property
    if (typeof current !== 'object' && typeof current !== 'function') { 
        // Note: functions shouldn't be in JSON but good guard
        return undefined;
    }
    current = (current as any)[k];
  }

  return current;
}

/**
 * Builds a nested JSON object from a dot-notation key path and a value.
 * Inverse of extractValue.
 *
 * @param keyPath The dot-notation key path (e.g. "sensor.temp")
 * @param value The value to embed
 * @returns A nested object (e.g. { sensor: { temp: 25 } })
 */
export function buildValue(keyPath: string, value: any): Record<string, any> {
  const keys = keyPath.split('.');
  const result: any = {};
  let current = result;
  for (let i = 0; i < keys.length - 1; i++) {
    current[keys[i]] = {};
    current = current[keys[i]];
  }
  current[keys[keys.length - 1]] = value;
  return result;
}
