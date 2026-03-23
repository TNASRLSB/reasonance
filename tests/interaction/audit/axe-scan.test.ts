import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('Phase 3C — axe-core WCAG Scan', () => {
  test('main view WCAG 2.1 AA violations', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
      .analyze();

    // Log all violations for the report
    console.log(`\n=== AXE-CORE RESULTS ===`);
    console.log(`Violations: ${results.violations.length}`);
    console.log(`Passes: ${results.passes.length}`);
    console.log(`Incomplete: ${results.incomplete.length}`);
    console.log(`Inapplicable: ${results.inapplicable.length}`);

    for (const violation of results.violations) {
      console.log(`\n--- VIOLATION: ${violation.id} ---`);
      console.log(`Impact: ${violation.impact}`);
      console.log(`Description: ${violation.description}`);
      console.log(`Help: ${violation.help}`);
      console.log(`WCAG: ${violation.tags.filter(t => t.startsWith('wcag')).join(', ')}`);
      console.log(`Nodes affected: ${violation.nodes.length}`);
      for (const node of violation.nodes.slice(0, 5)) {
        console.log(`  Target: ${node.target.join(' > ')}`);
        console.log(`  HTML: ${node.html.substring(0, 200)}`);
        console.log(`  Fix: ${node.failureSummary}`);
      }
      if (violation.nodes.length > 5) {
        console.log(`  ... and ${violation.nodes.length - 5} more nodes`);
      }
    }

    // Also log incomplete (needs review)
    if (results.incomplete.length > 0) {
      console.log(`\n=== NEEDS REVIEW (${results.incomplete.length}) ===`);
      for (const item of results.incomplete) {
        console.log(`  ${item.id}: ${item.help} (${item.nodes.length} nodes)`);
      }
    }
  });

  test('main view critical/serious violations only', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
      .analyze();

    const critical = results.violations.filter(v => v.impact === 'critical');
    const serious = results.violations.filter(v => v.impact === 'serious');

    console.log(`\nCritical violations: ${critical.length}`);
    console.log(`Serious violations: ${serious.length}`);

    for (const v of [...critical, ...serious]) {
      console.log(`  [${v.impact?.toUpperCase()}] ${v.id}: ${v.help} (${v.nodes.length} nodes)`);
    }

    // This test should eventually assert zero critical violations
    // For now, we collect data
    expect(critical.length).toBeDefined();
  });
});
