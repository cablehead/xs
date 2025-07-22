import { chromium } from 'playwright';

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  
  // Set viewport size
  await page.setViewportSize({ width: 1200, height: 800 });
  
  // Navigate to the page
  await page.goto('http://localhost:4321/xs/');
  
  // Wait for the page to load completely
  await page.waitForLoadState('networkidle');
  
  // Take screenshot
  const filename = process.argv[2] || 'current-splash-page.png';
  await page.screenshot({ 
    path: filename,
    fullPage: true
  });
  
  console.log(`Screenshot saved as ${filename}`);
  
  await browser.close();
})();