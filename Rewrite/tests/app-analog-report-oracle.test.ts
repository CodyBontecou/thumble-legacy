import { expect, test } from "bun:test";
import { isAppAnalogReport } from "../tooling/real-service-phone-probe";

test("actual OS analog oracle admits simultaneous bounded pixel rounding and exact neutral remainder", () => {
  const report = Buffer.from("0014001066663333000000003333000000000000", "hex");
  expect(isAppAnalogReport(report.toString("hex"))).toBe(true);
  const pixel = Buffer.from(report);
  pixel[4] = 105;
  pixel.writeInt16LE(13300, 6);
  expect(isAppAnalogReport(pixel.toString("hex"))).toBe(true);
  for (const index of [0, 1, 2, 3, 8, 10, 14, 19]) {
    const wrong = Buffer.from(report);
    wrong[index] = wrong[index]! + 1;
    expect(isAppAnalogReport(wrong.toString("hex"))).toBe(false);
  }
  for (const index of [4, 5, 6, 12]) {
    const missing = Buffer.from(report);
    missing[index] = 0;
    if (index >= 6) missing[index + 1] = 0;
    expect(isAppAnalogReport(missing.toString("hex"))).toBe(false);
  }
  expect(isAppAnalogReport("secret")).toBe(false);
  expect(isAppAnalogReport(report.toString("hex") + "00")).toBe(false);
});
