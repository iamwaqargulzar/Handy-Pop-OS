import { expect, test } from "@playwright/test";

const rgbChannels = (value: string): number[] =>
  value
    .match(/\d+(?:\.\d+)?/g)
    ?.slice(0, 3)
    .map(Number) ?? [];

test("starts with a dark neutral surface and cool accent", async ({ page }) => {
  await page.goto("/");

  const colors = await page.evaluate(() => {
    const rootStyles = getComputedStyle(document.documentElement);
    const accentProbe = document.createElement("span");
    accentProbe.style.color = "var(--color-logo-primary)";
    document.body.append(accentProbe);
    const accent = getComputedStyle(accentProbe).color;
    accentProbe.remove();

    return {
      background: rootStyles.backgroundColor,
      text: rootStyles.color,
      accent,
    };
  });

  const [backgroundRed, backgroundGreen, backgroundBlue] = rgbChannels(
    colors.background,
  );
  const [textRed, textGreen, textBlue] = rgbChannels(colors.text);
  const [accentRed, , accentBlue] = rgbChannels(colors.accent);

  expect(Math.max(backgroundRed, backgroundGreen, backgroundBlue)).toBeLessThan(
    40,
  );
  expect(Math.min(textRed, textGreen, textBlue)).toBeGreaterThan(200);
  expect(accentBlue).toBeGreaterThan(accentRed);
});
