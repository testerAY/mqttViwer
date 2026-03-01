import protobuf from 'protobufjs';
import type { ProtoSchema, ProtoField } from '../types/dashboard';

/**
 * Generates a .proto file text string from a ProtoSchema object.
 */
export function generateProtoText(schema: ProtoSchema): string {
  const lines: string[] = [];
  lines.push('syntax = "proto2";');
  lines.push('');
  if (schema.packageName) {
    lines.push(`package ${schema.packageName};`);
    lines.push('');
  }

  for (const msg of schema.messages) {
    lines.push(`message ${msg.name} {`);
    for (const field of msg.fields) {
      lines.push(`  ${field.rule} ${field.type} ${field.name} = ${field.id};`);
    }
    lines.push('}');
    lines.push('');
  }
  return lines.join('\n');
}

/**
 * Parses a .proto file text into a ProtoSchema object.
 * Handles proto2/proto3 syntax, simple message/field definitions.
 */
export function parseProtoText(protoText: string): ProtoSchema {
  const schema: ProtoSchema = {
    messages: [],
    rootMessage: '',
  };

  const pkgMatch = protoText.match(/^package\s+([\w.]+)\s*;/m);
  if (pkgMatch) schema.packageName = pkgMatch[1];

  // Extract messages with their content blocks
  const messageRegex = /message\s+(\w+)\s*\{([^}]*)\}/gs;
  let msgMatch: RegExpExecArray | null;

  while ((msgMatch = messageRegex.exec(protoText)) !== null) {
    const msgName = msgMatch[1];
    const body = msgMatch[2];
    const fields: ProtoField[] = [];

    // Parse field lines — handles both proto2 (with rule keyword) and proto3 (without rule keyword)
    // proto2: "required|optional|repeated type name = N;"
    // proto3: "type name = N;"
    const fieldRegex =
      /^\s*(?:(required|optional|repeated)\s+)?(\w+)\s+(\w+)\s*=\s*(\d+)\s*(?:\[.*?\])?\s*;/gm;
    let fieldMatch: RegExpExecArray | null;

    while ((fieldMatch = fieldRegex.exec(body)) !== null) {
      // Skip reserved keywords that are not field type declarations
      const keyword = fieldMatch[2];
      if (keyword === 'option' || keyword === 'reserved' || keyword === 'extensions') continue;

      fields.push({
        rule: (fieldMatch[1] as ProtoField['rule']) || 'optional',
        type: keyword as ProtoField['type'],
        name: fieldMatch[3],
        id: parseInt(fieldMatch[4], 10),
      });
    }

    schema.messages.push({ name: msgName, fields });
  }

  if (schema.messages.length > 0) {
    schema.rootMessage = schema.messages[0].name;
  }

  return schema;
}

/**
 * Builds a protobufjs Root from a ProtoSchema using in-memory JSON descriptor.
 */
function buildRoot(schema: ProtoSchema): protobuf.Root {
  const nested: Record<string, any> = {};

  for (const msg of schema.messages) {
    const fields: Record<string, any> = {};
    for (const f of msg.fields) {
      fields[f.name] = {
        rule: f.rule,
        type: f.type,
        id: f.id,
      };
    }
    nested[msg.name] = { fields };
  }

  const descriptor = schema.packageName
    ? { nested: { [schema.packageName]: { nested } } }
    : { nested };

  return protobuf.Root.fromJSON(descriptor);
}

/**
 * Decodes a base64-encoded binary payload using the provided ProtoSchema.
 * Returns a plain object on success, throws a descriptive Error on failure.
 */
export function decodeProtoPayload(
  base64Payload: string,
  schema: ProtoSchema,
): Record<string, any> {
  if (!schema.rootMessage) {
    throw new Error('ProtoSchema has no rootMessage defined.');
  }
  if (schema.messages.length === 0) {
    throw new Error('ProtoSchema has no message definitions.');
  }

  let binaryData: Uint8Array;
  try {
    const binaryStr = atob(base64Payload);
    binaryData = new Uint8Array(binaryStr.length);
    for (let i = 0; i < binaryStr.length; i++) {
      binaryData[i] = binaryStr.charCodeAt(i);
    }
  } catch (e) {
    throw new Error(`Failed to decode base64 payload: ${e}`);
  }

  try {
    const root = buildRoot(schema);
    const lookupName = schema.packageName
      ? `${schema.packageName}.${schema.rootMessage}`
      : schema.rootMessage;
    const MessageType = root.lookupType(lookupName);
    const decoded = MessageType.decode(binaryData);
    return MessageType.toObject(decoded, {
      longs: Number,
      enums: String,
      bytes: String,
      defaults: true,
      arrays: true,
    });
  } catch (e) {
    throw new Error(`Protobuf decode failed: ${e}`);
  }
}

/**
 * Returns the fields of the root message in the schema (for dropdown population).
 */
export function getRootMessageFields(schema: ProtoSchema): ProtoField[] {
  if (!schema.rootMessage) return [];
  const rootMsg = schema.messages.find(m => m.name === schema.rootMessage);
  return rootMsg?.fields ?? [];
}

/**
 * Coerces a value to the appropriate JavaScript type for a given ProtoField.
 */
export function coerceValueForField(field: ProtoField, value: any): any {
  switch (field.type) {
    case 'bool':
      return value === true || value === 1 || value === '1' || value === 'true' || value === 'ON';
    case 'int32': case 'int64': case 'uint32': case 'uint64':
    case 'sint32': case 'sint64': case 'fixed32': case 'fixed64':
    case 'sfixed32': case 'sfixed64':
      return Number(value);
    case 'float': case 'double':
      return parseFloat(String(value));
    case 'string':
      return String(value);
    default:
      return value;
  }
}

/**
 * Encodes field values into a Protobuf binary message and returns a base64 string.
 * Fields not included in fieldValues use Protobuf defaults (0, false, "").
 */
export function encodeProtoPayload(
  schema: ProtoSchema,
  fieldValues: Record<string, any>,
): string {
  if (!schema.rootMessage) throw new Error('ProtoSchema has no rootMessage defined.');

  const root = buildRoot(schema);
  const lookupName = schema.packageName
    ? `${schema.packageName}.${schema.rootMessage}`
    : schema.rootMessage;
  const MessageType = root.lookupType(lookupName);

  const rootMsg = schema.messages.find(m => m.name === schema.rootMessage);
  const msgData: Record<string, any> = {};
  for (const [fieldName, value] of Object.entries(fieldValues)) {
    const fieldDef = rootMsg?.fields.find(f => f.name === fieldName);
    msgData[fieldName] = fieldDef ? coerceValueForField(fieldDef, value) : value;
  }

  const errMsg = MessageType.verify(msgData);
  if (errMsg) throw new Error(`Protobuf encode verify failed: ${errMsg}`);

  const encoded = MessageType.encode(MessageType.create(msgData)).finish();
  let binary = '';
  for (let i = 0; i < encoded.length; i++) binary += String.fromCharCode(encoded[i]);
  return btoa(binary);
}

/**
 * Validates a ProtoSchema for common issues.
 * Returns an array of error strings (empty array = valid).
 */
export function validateProtoSchema(schema: ProtoSchema): string[] {
  const errors: string[] = [];

  if (schema.messages.length === 0) {
    errors.push('Schema must define at least one message.');
  }

  if (!schema.rootMessage) {
    errors.push('A root message must be selected.');
  } else {
    const rootExists = schema.messages.some(m => m.name === schema.rootMessage);
    if (!rootExists) {
      errors.push(`Root message "${schema.rootMessage}" is not defined in the schema.`);
    }
  }

  const msgNames = new Set<string>();
  for (const msg of schema.messages) {
    if (!msg.name || !/^\w+$/.test(msg.name)) {
      errors.push(`Invalid message name: "${msg.name}"`);
    }
    if (msgNames.has(msg.name)) {
      errors.push(`Duplicate message name: "${msg.name}"`);
    }
    msgNames.add(msg.name);

    const fieldIds = new Set<number>();
    const fieldNames = new Set<string>();
    for (const f of msg.fields) {
      if (!f.name || !/^\w+$/.test(f.name)) {
        errors.push(`Message "${msg.name}": invalid field name "${f.name}"`);
      }
      if (fieldNames.has(f.name)) {
        errors.push(`Message "${msg.name}": duplicate field name "${f.name}"`);
      }
      fieldNames.add(f.name);

      if (f.id < 1 || f.id > 536870911) {
        errors.push(
          `Message "${msg.name}", field "${f.name}": field number must be between 1 and 536870911`,
        );
      }
      if (fieldIds.has(f.id)) {
        errors.push(`Message "${msg.name}": duplicate field number ${f.id}`);
      }
      fieldIds.add(f.id);
    }
  }

  return errors;
}
