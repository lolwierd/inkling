import { expect, test } from "bun:test";
import { boundedPage } from "../src/web.ts";

test("chunked pages stop at the byte budget and cancel the producer", async () => {
  let produced = 0, cancelled = false;
  const body = new ReadableStream<Uint8Array>({
    pull(controller) { produced++; controller.enqueue(new Uint8Array(8)); },
    cancel() { cancelled = true; },
  });
  expect(await boundedPage(new Response(body), 20)).toBeNull();
  expect(cancelled).toBe(true);
  expect(produced).toBeLessThanOrEqual(4);
});

test("the budget counts UTF-8 bytes, and decoding spans chunk boundaries", async () => {
  const bytes = new TextEncoder().encode("é🧵");
  const response = () => new Response(new ReadableStream({
    start(c) { c.enqueue(bytes.slice(0, 1)); c.enqueue(bytes.slice(1)); c.close(); },
  }));
  expect(await boundedPage(response(), bytes.length)).toBe("é🧵");
  expect(await boundedPage(response(), bytes.length - 1)).toBeNull();
});

test("a broken stream fails without retaining its reader lock", async () => {
  const body = new ReadableStream<Uint8Array>({ start(c) { c.error(new Error("disconnected")); } });
  await expect(boundedPage(new Response(body))).rejects.toThrow("disconnected");
  expect(body.locked).toBe(false);
});
