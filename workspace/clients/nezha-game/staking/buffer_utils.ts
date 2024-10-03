import BN from "bn.js";

export function u8(x: number): Buffer {
  let b = Buffer.alloc(1);
  b.writeUInt8(x);
  return b;
}

export function u64(x: number): Buffer {
  if (x < 0) {
    throw Error('Negative numbers are not allowed');
  }
  let x_ = BigInt(x);

  let b = Buffer.alloc(8);
  b.writeBigUInt64LE(x_);
  return b;
}

export function u32(x: number): Buffer {
  if (x < 0) {
    throw Error('Negative numbers are not allowed');
  }

  let b = Buffer.alloc(4);
  b.writeUInt32LE(x);
  return b;
}

export function i64(x: number): Buffer {
  let x_ = BigInt(x);

  let b = Buffer.alloc(8);
  b.writeBigInt64LE(x_);
  return b;
}

export function i64BN(x: BN): Buffer {
  let b = x.toBuffer('le', 8);
  return b;
}

export function concatBufs(data: Buffer[]): Buffer {
  let totalLen = 0;
  for (let b of data) {
    totalLen += b.length;
  }

  let buf = Buffer.alloc(totalLen);
  let offset = 0;
  for (let b of data) {
    b.copy(buf, offset);
    offset += b.length;
  }

  return buf;
}
