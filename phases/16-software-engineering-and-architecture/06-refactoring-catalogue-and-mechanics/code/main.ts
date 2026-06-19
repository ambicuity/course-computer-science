/**
 * Refactoring Catalogue and Mechanics — Phase 16, Lesson 06
 * A step-by-step refactoring kata in TypeScript.
 * Each version is complete and produces identical output.
 */

// =============================================================================
// VERSION 0: SMELLY CODE (before refactoring)
// =============================================================================

function calculateFeeV0(
  vehicleType: string,
  days: number,
  miles: number,
  insurance: boolean,
  discount: number
): number {
  let t = 0;
  if (vehicleType === "SEDAN") {
    const base = 30;
    t += base * days;
    t += miles * 0.15;
    if (miles > 500) {
      t -= (miles - 500) * 0.05;
    }
    if (insurance) {
      t += days * 10;
    }
    t = t * (1 - discount);
  } else if (vehicleType === "SUV") {
    const base = 50;
    t += base * days;
    t += miles * 0.2;
    if (miles > 300) {
      t -= (miles - 300) * 0.08;
    }
    if (insurance) {
      t += days * 15;
    }
    t = t * (1 - discount);
  } else if (vehicleType === "VAN") {
    const base = 45;
    t += base * days;
    t += miles * 0.18;
    if (miles > 400) {
      t -= (miles - 400) * 0.06;
    }
    if (insurance) {
      t += days * 12;
    }
    t = t * (1 - discount);
  } else if (vehicleType === "MOTORCYCLE") {
    const base = 20;
    t += base * days;
    t += miles * 0.1;
    if (miles > 200) {
      t -= (miles - 200) * 0.03;
    }
    if (insurance) {
      t += days * 8;
    }
    t = t * (1 - discount);
  } else {
    throw new Error(`Unknown vehicle type: ${vehicleType}`);
  }
  return Math.round(t * 100) / 100;
}

function printReceiptV0(
  vehicleType: string,
  days: number,
  miles: number,
  insurance: boolean,
  discount: number
): void {
  console.log("=== Rental Receipt (V0: Smelly) ===");
  console.log(`Vehicle: ${vehicleType}`);
  console.log(`Days: ${days}, Miles: ${miles}`);
  console.log(`Insurance: ${insurance ? "Yes" : "No"}`);
  console.log(`Discount: ${(discount * 100).toFixed(0)}%`);
  console.log(`Total: $${calculateFeeV0(vehicleType, days, miles, insurance, discount).toFixed(2)}`);
  console.log();
}

// =============================================================================
// VERSION 1: EXTRACT METHOD + EXTRACT VARIABLE + RENAME
// =============================================================================

const BASE_RATES: Record<string, number> = {
  SEDAN: 30, SUV: 50, VAN: 45, MOTORCYCLE: 20,
};

const PER_MILE_RATES: Record<string, number> = {
  SEDAN: 0.15, SUV: 0.20, VAN: 0.18, MOTORCYCLE: 0.10,
};

const MILEAGE_THRESHOLDS: Record<string, number> = {
  SEDAN: 500, SUV: 300, VAN: 400, MOTORCYCLE: 200,
};

const MILEAGE_DISCOUNT_RATES: Record<string, number> = {
  SEDAN: 0.05, SUV: 0.08, VAN: 0.06, MOTORCYCLE: 0.03,
};

const DAILY_INSURANCE_COSTS: Record<string, number> = {
  SEDAN: 10, SUV: 15, VAN: 12, MOTORCYCLE: 8,
};

function calculateMileageCharge(vehicleType: string, miles: number): number {
  const perMile = PER_MILE_RATES[vehicleType];
  const threshold = MILEAGE_THRESHOLDS[vehicleType];
  const discountRate = MILEAGE_DISCOUNT_RATES[vehicleType];
  let rawCharge = miles * perMile;
  if (miles > threshold) {
    rawCharge -= (miles - threshold) * discountRate;
  }
  return rawCharge;
}

function calculateFeeV1(
  vehicleType: string, days: number, miles: number, insurance: boolean, discount: number
): number {
  const dailyBase = BASE_RATES[vehicleType];
  const mileage = calculateMileageCharge(vehicleType, miles);
  let total = dailyBase * days + mileage;
  if (insurance) {
    total += DAILY_INSURANCE_COSTS[vehicleType] * days;
  }
  total = total * (1 - discount);
  return Math.round(total * 100) / 100;
}

function printReceiptV1(
  vehicleType: string, days: number, miles: number, insurance: boolean, discount: number
): void {
  console.log("=== Rental Receipt (V1: Extract Method) ===");
  console.log(`Vehicle: ${vehicleType}`);
  console.log(`Days: ${days}, Miles: ${miles}`);
  console.log(`Insurance: ${insurance ? "Yes" : "No"}`);
  console.log(`Discount: ${(discount * 100).toFixed(0)}%`);
  console.log(`Total: $${calculateFeeV1(vehicleType, days, miles, insurance, discount).toFixed(2)}`);
  console.log();
}

// =============================================================================
// VERSION 2: REPLACE CONDITIONAL WITH POLYMORPHISM + INTRODUCE PARAMETER OBJECT
// =============================================================================

interface RentalParams {
  days: number;
  miles: number;
  insurance: boolean;
  discount: number;
}

abstract class VehicleRental {
  abstract baseRate(): number;
  abstract perMileRate(): number;
  abstract mileageThreshold(): number;
  abstract mileageDiscountRate(): number;
  abstract dailyInsurance(): number;

  calculateMileageCharge(rental: RentalParams): number {
    let raw = rental.miles * this.perMileRate();
    if (rental.miles > this.mileageThreshold()) {
      raw -= (rental.miles - this.mileageThreshold()) * this.mileageDiscountRate();
    }
    return raw;
  }

  calculateFee(rental: RentalParams): number {
    const baseCharge = this.baseRate() * rental.days;
    const mileage = this.calculateMileageCharge(rental);
    let total = baseCharge + mileage;
    if (rental.insurance) {
      total += this.dailyInsurance() * rental.days;
    }
    total = total * (1 - rental.discount);
    return Math.round(total * 100) / 100;
  }
}

class SedanRental extends VehicleRental {
  baseRate() { return 30; }
  perMileRate() { return 0.15; }
  mileageThreshold() { return 500; }
  mileageDiscountRate() { return 0.05; }
  dailyInsurance() { return 10; }
}

class SUVRental extends VehicleRental {
  baseRate() { return 50; }
  perMileRate() { return 0.20; }
  mileageThreshold() { return 300; }
  mileageDiscountRate() { return 0.08; }
  dailyInsurance() { return 15; }
}

class VanRental extends VehicleRental {
  baseRate() { return 45; }
  perMileRate() { return 0.18; }
  mileageThreshold() { return 400; }
  mileageDiscountRate() { return 0.06; }
  dailyInsurance() { return 12; }
}

class MotorcycleRental extends VehicleRental {
  baseRate() { return 20; }
  perMileRate() { return 0.10; }
  mileageThreshold() { return 200; }
  mileageDiscountRate() { return 0.03; }
  dailyInsurance() { return 8; }
}

const VEHICLE_MAP: Record<string, new () => VehicleRental> = {
  SEDAN: SedanRental,
  SUV: SUVRental,
  VAN: VanRental,
  MOTORCYCLE: MotorcycleRental,
};

function makeVehicle(vehicleType: string): VehicleRental {
  const Cls = VEHICLE_MAP[vehicleType];
  if (!Cls) throw new Error(`Unknown vehicle type: ${vehicleType}`);
  return new Cls();
}

function calculateFeeV2(
  vehicleType: string, days: number, miles: number, insurance: boolean, discount: number
): number {
  const rental: RentalParams = { days, miles, insurance, discount };
  const vehicle = makeVehicle(vehicleType);
  return vehicle.calculateFee(rental);
}

function printReceiptV2(
  vehicleType: string, days: number, miles: number, insurance: boolean, discount: number
): void {
  console.log("=== Rental Receipt (V2: Polymorphism + Parameter Object) ===");
  console.log(`Vehicle: ${vehicleType}`);
  console.log(`Days: ${days}, Miles: ${miles}`);
  console.log(`Insurance: ${insurance ? "Yes" : "No"}`);
  console.log(`Discount: ${(discount * 100).toFixed(0)}%`);
  console.log(`Total: $${calculateFeeV2(vehicleType, days, miles, insurance, discount).toFixed(2)}`);
  console.log();
}

// =============================================================================
// VERSION 3: REPLACE TEMP WITH QUERY (final polished version)
// =============================================================================

class RentalCalculation {
  private vehicle: VehicleRental;
  private rental: RentalParams;

  constructor(vehicle: VehicleRental, rental: RentalParams) {
    this.vehicle = vehicle;
    this.rental = rental;
  }

  get days(): number { return this.rental.days; }
  get miles(): number { return this.rental.miles; }
  get hasInsurance(): boolean { return this.rental.insurance; }
  get discountFraction(): number { return this.rental.discount; }

  get baseCharge(): number {
    return this.vehicle.baseRate() * this.days;
  }

  get mileageCharge(): number {
    return this.vehicle.calculateMileageCharge(this.rental);
  }

  get insuranceCharge(): number {
    return this.hasInsurance ? this.vehicle.dailyInsurance() * this.days : 0;
  }

  get subtotal(): number {
    return this.baseCharge + this.mileageCharge + this.insuranceCharge;
  }

  get total(): number {
    return Math.round(this.subtotal * (1 - this.discountFraction) * 100) / 100;
  }
}

function printReceiptFinal(
  vehicleType: string, days: number, miles: number, insurance: boolean, discount: number
): void {
  const vehicle = makeVehicle(vehicleType);
  const rental: RentalParams = { days, miles, insurance, discount };
  const calc = new RentalCalculation(vehicle, rental);

  console.log("=== Rental Receipt (V3: Replace Temp with Query) ===");
  console.log(`Vehicle: ${vehicleType}`);
  console.log(`Days: ${calc.days}, Miles: ${calc.miles}`);
  console.log(`Insurance: ${calc.hasInsurance ? "Yes" : "No"}`);
  console.log(`Discount: ${(calc.discountFraction * 100).toFixed(0)}%`);
  console.log(`  Base charge:   $${calc.baseCharge.toFixed(2)}`);
  console.log(`  Mileage:       $${calc.mileageCharge.toFixed(2)}`);
  console.log(`  Insurance:     $${calc.insuranceCharge.toFixed(2)}`);
  console.log(`  Subtotal:      $${calc.subtotal.toFixed(2)}`);
  console.log(`  Total:         $${calc.total.toFixed(2)}`);
  console.log();
}

// =============================================================================
// MAIN: Run all versions side by side
// =============================================================================

const TEST_CASES: [string, number, number, boolean, number][] = [
  ["SEDAN", 3, 250, false, 0.0],
  ["SEDAN", 5, 600, true, 0.10],
  ["SUV", 2, 350, true, 0.05],
  ["VAN", 7, 500, false, 0.15],
  ["MOTORCYCLE", 1, 100, true, 0.0],
  ["MOTORCYCLE", 4, 250, true, 0.20],
];

function main(): void {
  console.log("=".repeat(60));
  console.log("REFACTORING KATA: Before / After Comparison");
  console.log("=".repeat(60));
  console.log();

  console.log("--- V0: Smelly code (long method, feature envy, switch) ---");
  for (const [vehicleType, days, miles, insurance, discount] of TEST_CASES) {
    printReceiptV0(vehicleType, days, miles, insurance, discount);
  }

  console.log("--- V1: After Extract Method + Extract Variable + Rename ---");
  for (const [vehicleType, days, miles, insurance, discount] of TEST_CASES) {
    printReceiptV1(vehicleType, days, miles, insurance, discount);
  }

  console.log("--- V2: After Replace Conditional with Polymorphism + Parameter Object ---");
  for (const [vehicleType, days, miles, insurance, discount] of TEST_CASES) {
    printReceiptV2(vehicleType, days, miles, insurance, discount);
  }

  console.log("--- V3: After Replace Temp with Query + Move Method (final) ---");
  for (const [vehicleType, days, miles, insurance, discount] of TEST_CASES) {
    printReceiptFinal(vehicleType, days, miles, insurance, discount);
  }

  console.log("=".repeat(60));
  console.log("VERIFICATION: All versions produce identical totals");
  console.log("=".repeat(60));

  let allMatch = true;
  for (const [vehicleType, days, miles, insurance, discount] of TEST_CASES) {
    const v0 = calculateFeeV0(vehicleType, days, miles, insurance, discount);
    const v1 = calculateFeeV1(vehicleType, days, miles, insurance, discount);
    const v2 = calculateFeeV2(vehicleType, days, miles, insurance, discount);
    const v3 = new RentalCalculation(
      makeVehicle(vehicleType),
      { days, miles, insurance, discount }
    ).total;

    const match = v0 === v1 && v1 === v2 && v2 === v3;
    const status = match ? "PASS" : "FAIL";
    console.log(
      `  ${vehicleType.padEnd(12)} d=${days} m=${miles.toString().padStart(5)} ` +
      `ins=${insurance.toString().padEnd(5)} disc=${discount.toFixed(2)}  ` +
      `v0=${v0.toFixed(2)} v1=${v1.toFixed(2)} v2=${v2.toFixed(2)} v3=${v3.toFixed(2)}  [${status}]`
    );
    if (!match) allMatch = false;
  }

  console.log();
  if (allMatch) {
    console.log("All versions produce identical output. Refactoring preserved behavior!");
  } else {
    console.log("MISMATCH DETECTED — refactoring changed behavior!");
  }
}

main();