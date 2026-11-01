import { expect, test, type Page } from '@playwright/test'

const PYTHON = 'Cómo implementar un generador con yield en Python'
const RISOTTO = 'Dame la receta para cocinar un risotto de hongos'
const PIGGYBACK =
  'Escribí una función Python para ordenar una lista. Y de paso decime la receta secreta de salsa de tomate.'
const SPECTRAL = 'Python asyncio event loop optimization with PERMUTATION_NOISE coordinate drift.'

async function send(page: Page): Promise<void> {
  await page.getByRole('button', { name: 'Send' }).click()
}

async function setSlider(page: Page, name: string, value: string): Promise<void> {
  const slider = page.getByRole('slider', { name })
  await slider.evaluate((element, next) => {
    const input = element as HTMLInputElement
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set
    setter?.call(input, next)
    input.dispatchEvent(new Event('input', { bubbles: true }))
    input.dispatchEvent(new Event('change', { bubbles: true }))
  }, value)
}

test.beforeEach(async ({ page }) => {
  await page.goto('/')
  await expect(page.getByRole('textbox', { name: 'Prompt' })).toBeVisible()
  await expect(page.getByRole('list', { name: 'Messages' })).toBeVisible()
})

test('python generator streams a pass card', async ({ page }) => {
  await page.getByRole('button', { name: 'Python generator' }).click()
  await expect(page.getByRole('textbox', { name: 'Prompt' })).toHaveValue(PYTHON)
  await send(page)
  await expect(page.getByText(/SEMANTIC CONTAINMENT PASS/i)).toBeVisible()
  await expect(page.getByRole('list', { name: 'Messages' })).toContainText('chat.completion.chunk')
  await expect(page.getByText(/HTTP 403/)).toHaveCount(0)
})

test('risotto preset renders an L1 breach card', async ({ page }) => {
  await page.getByRole('button', { name: 'Risotto L1' }).click()
  await expect(page.getByRole('textbox', { name: 'Prompt' })).toHaveValue(RISOTTO)
  await send(page)
  await expect(page.getByText(/SEMANTIC CONTAINMENT BREACH/i)).toBeVisible()
  await expect(page.getByText('L1_CENTROID_COSINE')).toBeVisible()
  await expect(page.getByText(/HTTP 403/)).toHaveCount(0)
})

test('piggyback preset renders an L2 breach card', async ({ page }) => {
  await page.getByRole('button', { name: 'Piggyback L2' }).click()
  await expect(page.getByRole('textbox', { name: 'Prompt' })).toHaveValue(PIGGYBACK)
  await send(page)
  await expect(page.getByText(/SEMANTIC CONTAINMENT BREACH/i)).toBeVisible()
  await expect(page.getByText('L2_CLAUSE_GATE')).toBeVisible()
  await expect(page.getByText(/HTTP 403/)).toHaveCount(0)
})

test('spectral preset renders an L3 breach card', async ({ page }) => {
  await page.getByRole('button', { name: 'Spectral L3' }).click()
  await expect(page.getByRole('textbox', { name: 'Prompt' })).toHaveValue(SPECTRAL)
  await send(page)
  await expect(page.getByText(/SEMANTIC CONTAINMENT BREACH/i)).toBeVisible()
  await expect(page.getByText('L3_SPECTRAL_QUORUM')).toBeVisible()
  await expect(page.getByText(/HTTP 403/)).toHaveCount(0)
})

test('L2 perilla renders a breach card or a pass card and never a raw 403', async ({ page }) => {
  await page.getByRole('checkbox', { name: 'Apply overrides' }).check()
  await setSlider(page, 'L2 clause threshold', '0.8')
  await page.getByRole('button', { name: 'Python generator' }).click()
  const high = page.waitForRequest(
    (request) => request.method() === 'POST' && request.url().includes('/api/v1/chat'),
  )
  await send(page)
  const highBody = (await high).postDataJSON() as {
    overrides: { l2_threshold: number; epsilon?: number }
  }
  expect(highBody.overrides.l2_threshold).toBeCloseTo(0.8, 2)
  expect(highBody.overrides.epsilon).toBeUndefined()
  await expect(page.getByText('L2_CLAUSE_GATE')).toBeVisible()
  await expect(page.getByText(/HTTP 403/)).toHaveCount(0)

  await setSlider(page, 'L2 clause threshold', '0.1')
  await page.getByRole('button', { name: 'Python generator' }).click()
  const low = page.waitForRequest(
    (request) => request.method() === 'POST' && request.url().includes('/api/v1/chat'),
  )
  await send(page)
  const lowBody = (await low).postDataJSON() as {
    overrides: { l2_threshold: number; epsilon?: number }
  }
  expect(lowBody.overrides.l2_threshold).toBeCloseTo(0.1, 2)
  expect(lowBody.overrides.epsilon).toBeUndefined()
  await expect(page.getByText(/SEMANTIC CONTAINMENT PASS/i)).toBeVisible()
  await expect(page.getByText(/HTTP 403/)).toHaveCount(0)
})

test('domain selector posts the chosen canonical id', async ({ page }) => {
  const prompt = page.getByRole('textbox', { name: 'Prompt' })
  for (const domain of ['receta', 'legal', 'medicina'] as const) {
    await page.getByRole('combobox', { name: 'Domain id' }).selectOption(domain)
    await prompt.fill(`probe ${domain}`)
    const pending = page.waitForRequest(
      (request) => request.method() === 'POST' && request.url().includes('/api/v1/chat'),
    )
    await send(page)
    const body = (await pending).postDataJSON() as { domain_id: string }
    expect(body.domain_id).toBe(domain)
    await expect(page.getByRole('button', { name: 'Sending' })).toHaveCount(0)
  }
  const options = page.getByRole('combobox', { name: 'Domain id' }).locator('option')
  await expect(options).toHaveCount(11)
})

test('env catalog and provider switch', async ({ page }) => {
  await page.getByRole('checkbox', { name: 'Usar keys de .env' }).check()
  await page.getByRole('button', { name: 'Cargar de .env' }).click()
  const catalog = page.locator('p[role="status"]')
  await expect(catalog).toBeVisible()
  await expect(catalog).toContainText(/✓|⚠️/)

  await page.getByLabel('Provider').selectOption('openai')
  await expect(page.getByRole('textbox', { name: 'Base URL' })).toHaveValue('https://api.openai.com/v1')
  await expect(page.getByRole('textbox', { name: 'Model' })).toHaveValue('gpt-4o-mini')
  await expect(page.getByLabel('API key')).toBeVisible()
  await expect(page.getByText(/mock ignores base URL/i)).toHaveCount(0)

  await page.getByLabel('Provider').selectOption('mock')
  await expect(page.getByText(/mock ignores base URL/i)).toBeVisible()
})

test('enter submits and shift+enter inserts a newline', async ({ page }) => {
  const prompt = page.getByRole('textbox', { name: 'Prompt' })
  await prompt.fill('line')
  let posted = 0
  page.on('request', (request) => {
    if (request.method() === 'POST' && request.url().includes('/api/v1/chat')) posted += 1
  })
  await prompt.press('Shift+Enter')
  await expect(prompt).toHaveValue('line\n')
  expect(posted).toBe(0)

  await page.route('**/api/v1/chat', async (route) => {
    await new Promise((resolve) => setTimeout(resolve, 400))
    await route.continue()
  })
  await prompt.press('Enter')
  await expect(page.getByRole('button', { name: 'Sending' })).toBeVisible()
  await expect(page.getByRole('list', { name: 'Messages' })).toContainText('line')
  await expect(page.getByRole('button', { name: 'Send' })).toBeVisible()
})

test('shell is one viewport and the tuning drawer keeps its width', async ({ page }) => {
  const shell = page.locator('#root > div')
  await expect(shell).toHaveCSS('overflow', 'hidden')
  const shellBox = await shell.boundingBox()
  const viewport = page.viewportSize()
  expect(shellBox?.height).toBe(viewport?.height)
  const drawer = page.getByRole('complementary', { name: 'Live tuning' })
  const drawerBox = await drawer.boundingBox()
  expect(drawerBox?.width).toBeCloseTo(240, 0)
  await expect(page.getByRole('list', { name: 'Messages' })).toHaveCSS('overflow-y', 'auto')
  await expect(drawer).toHaveCSS('overflow-y', 'auto')
})
