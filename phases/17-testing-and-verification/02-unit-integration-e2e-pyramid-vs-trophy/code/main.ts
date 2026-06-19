type RiskProfile = {
  logic: number;
  boundary: number;
  journey: number;
  changeFrequency: number;
  runtimeBudgetMinutes: number;
};

type Allocation = {
  unit: number;
  integration: number;
  e2e: number;
};

function normalize(a: Allocation): [number, number, number] {
  const total = a.unit + a.integration + a.e2e;
  if (total <= 0) throw new Error("allocation total must be positive");
  return [a.unit / total, a.integration / total, a.e2e / total];
}

function utility(profile: RiskProfile, allocation: Allocation): number {
  const [u, i, e] = normalize(allocation);

  const coverage =
    profile.logic * (0.9 * u + 0.2 * i + 0.1 * e) +
    profile.boundary * (0.2 * u + 0.9 * i + 0.4 * e) +
    profile.journey * (0.1 * u + 0.4 * i + 1.0 * e);

  const runtimePenalty = 0.8 * u + 2.2 * i + 5.0 * e;
  const flakePenalty = profile.changeFrequency * (0.03 * u + 0.08 * i + 0.22 * e);
  const budgetPressure = Math.max(0, runtimePenalty - profile.runtimeBudgetMinutes / 4.0);

  return coverage - 0.1 * runtimePenalty - 1.3 * flakePenalty - 0.2 * budgetPressure;
}

function recommend(profile: RiskProfile): Allocation {
  const candidates: Allocation[] = [
    { unit: 70, integration: 20, e2e: 10 },
    { unit: 60, integration: 30, e2e: 10 },
    { unit: 55, integration: 35, e2e: 10 },
    { unit: 50, integration: 35, e2e: 15 },
    { unit: 45, integration: 40, e2e: 15 },
    { unit: 40, integration: 45, e2e: 15 },
  ];

  let best = candidates[0];
  let bestScore = utility(profile, best);
  for (const c of candidates.slice(1)) {
    const score = utility(profile, c);
    if (score > bestScore) {
      best = c;
      bestScore = score;
    }
  }
  return best;
}

function printScenario(name: string, profile: RiskProfile): void {
  const alloc = recommend(profile);
  const score = utility(profile, alloc);
  const [u, i, e] = normalize(alloc);
  console.log(`Scenario: ${name}`);
  console.log(`  Recommended split: unit=${alloc.unit} integration=${alloc.integration} e2e=${alloc.e2e}`);
  console.log(`  Normalized: unit=${(u * 100).toFixed(2)}% integration=${(i * 100).toFixed(2)}% e2e=${(e * 100).toFixed(2)}%`);
  console.log(`  Utility score: ${score.toFixed(4)}`);
}

const checkout: RiskProfile = {
  logic: 0.7,
  boundary: 0.9,
  journey: 0.8,
  changeFrequency: 0.6,
  runtimeBudgetMinutes: 12,
};

const libraryModule: RiskProfile = {
  logic: 0.9,
  boundary: 0.35,
  journey: 0.2,
  changeFrequency: 0.4,
  runtimeBudgetMinutes: 8,
};

printScenario("checkout", checkout);
printScenario("library-module", libraryModule);
