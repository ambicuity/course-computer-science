"""
Refactoring Catalogue and Mechanics — Phase 16, Lesson 06
A step-by-step refactoring kata: start with smelly code, apply refactorings one by one.
Each version is complete and produces identical output.
"""

from __future__ import annotations
from dataclasses import dataclass
from abc import ABC, abstractmethod
from typing import List


# =============================================================================
# VERSION 0: SMELLY CODE (before refactoring)
# =============================================================================

def calculate_fee_v0(vehicle_type: str, days: int, miles: float, insurance: bool, discount: float) -> float:
    """
    Kitchen-sink function with long method, feature envy, switch statements,
    duplicated logic, and cryptic variable names.
    """
    t = 0
    if vehicle_type == "SEDAN":
        base = 30
        t += base * days
        t += miles * 0.15
        if miles > 500:
            t -= (miles - 500) * 0.05
        if insurance:
            t += days * 10
        t = t * (1 - discount)
    elif vehicle_type == "SUV":
        base = 50
        t += base * days
        t += miles * 0.20
        if miles > 300:
            t -= (miles - 300) * 0.08
        if insurance:
            t += days * 15
        t = t * (1 - discount)
    elif vehicle_type == "VAN":
        base = 45
        t += base * days
        t += miles * 0.18
        if miles > 400:
            t -= (miles - 400) * 0.06
        if insurance:
            t += days * 12
        t = t * (1 - discount)
    elif vehicle_type == "MOTORCYCLE":
        base = 20
        t += base * days
        t += miles * 0.10
        if miles > 200:
            t -= (miles - 200) * 0.03
        if insurance:
            t += days * 8
        t = t * (1 - discount)
    else:
        raise ValueError(f"Unknown vehicle type: {vehicle_type}")
    return round(t, 2)


def print_receipt_v0(vehicle_type: str, days: int, miles: float, insurance: bool, discount: float) -> None:
    """Smelly receipt printer — duplicated logic from calculate_fee_v0."""
    print(f"=== Rental Receipt (V0: Smelly) ===")
    print(f"Vehicle: {vehicle_type}")
    print(f"Days: {days}, Miles: {miles}")
    print(f"Insurance: {'Yes' if insurance else 'No'}")
    print(f"Discount: {discount * 100:.0f}%")
    print(f"Total: ${calculate_fee_v0(vehicle_type, days, miles, insurance, discount):.2f}")
    print()


# =============================================================================
# VERSION 1: EXTRACT METHOD + EXTRACT VARIABLE + RENAME
# =============================================================================

def base_rate(vehicle_type: str) -> float:
    """Extracted: base daily rate per vehicle type."""
    rates = {"SEDAN": 30, "SUV": 50, "VAN": 45, "MOTORCYCLE": 20}
    return rates[vehicle_type]


def per_mile_rate(vehicle_type: str) -> float:
    """Extracted: cost per mile per vehicle type."""
    rates = {"SEDAN": 0.15, "SUV": 0.20, "VAN": 0.18, "MOTORCYCLE": 0.10}
    return rates[vehicle_type]


def mileage_discount_threshold(vehicle_type: str) -> float:
    """Extracted: mileage threshold above which a discount applies."""
    thresholds = {"SEDAN": 500, "SUV": 300, "VAN": 400, "MOTORCYCLE": 200}
    return thresholds[vehicle_type]


def mileage_discount_rate(vehicle_type: str) -> float:
    """Extracted: per-mile discount rate above threshold."""
    rates = {"SEDAN": 0.05, "SUV": 0.08, "VAN": 0.06, "MOTORCYCLE": 0.03}
    return rates[vehicle_type]


def daily_insurance_cost(vehicle_type: str) -> float:
    """Extracted: daily insurance add-on per vehicle type."""
    costs = {"SEDAN": 10, "SUV": 15, "VAN": 12, "MOTORCYCLE": 8}
    return costs[vehicle_type]


def calculate_mileage_charge(vehicle_type: str, miles: float) -> float:
    """Extracted: mileage charge with high-mileage discount."""
    per_mile = per_mile_rate(vehicle_type)
    threshold = mileage_discount_threshold(vehicle_type)
    discount_rate = mileage_discount_rate(vehicle_type)
    raw_charge = miles * per_mile
    if miles > threshold:
        raw_charge -= (miles - threshold) * discount_rate
    return raw_charge


def calculate_fee_v1(vehicle_type: str, days: int, miles: float, insurance: bool, discount: float) -> float:
    """After Extract Method + Extract Variable. Still has switch logic, but methods are named."""
    daily_base = base_rate(vehicle_type)
    mileage = calculate_mileage_charge(vehicle_type, miles)
    total = daily_base * days + mileage
    if insurance:
        total += daily_insurance_cost(vehicle_type) * days
    total = total * (1 - discount)
    return round(total, 2)


def print_receipt_v1(vehicle_type: str, days: int, miles: float, insurance: bool, discount: float) -> None:
    print(f"=== Rental Receipt (V1: Extract Method) ===")
    print(f"Vehicle: {vehicle_type}")
    print(f"Days: {days}, Miles: {miles}")
    print(f"Insurance: {'Yes' if insurance else 'No'}")
    print(f"Discount: {discount * 100:.0f}%")
    print(f"Total: ${calculate_fee_v1(vehicle_type, days, miles, insurance, discount):.2f}")
    print()


# =============================================================================
# VERSION 2: REPLACE CONDITIONAL WITH POLYMORPHISM
# =============================================================================

@dataclass
class Rental:
    """Parameter object grouping rental details."""
    days: int
    miles: float
    insurance: bool
    discount: float


class VehicleRental(ABC):
    """Abstract base — each vehicle type implements its own rate logic."""

    @abstractmethod
    def base_rate(self) -> float: ...

    @abstractmethod
    def per_mile_rate(self) -> float: ...

    @abstractmethod
    def mileage_threshold(self) -> float: ...

    @abstractmethod
    def mileage_discount_rate(self) -> float: ...

    @abstractmethod
    def daily_insurance(self) -> float: ...

    def calculate_mileage_charge(self, rental: Rental) -> float:
        raw = rental.miles * self.per_mile_rate()
        if rental.miles > self.mileage_threshold():
            raw -= (rental.miles - self.mileage_threshold()) * self.mileage_discount_rate()
        return raw

    def calculate_fee(self, rental: Rental) -> float:
        base_charge = self.base_rate() * rental.days
        mileage = self.calculate_mileage_charge(rental)
        total = base_charge + mileage
        if rental.insurance:
            total += self.daily_insurance() * rental.days
        total = total * (1 - rental.discount)
        return round(total, 2)


class SedanRental(VehicleRental):
    def base_rate(self) -> float: return 30
    def per_mile_rate(self) -> float: return 0.15
    def mileage_threshold(self) -> float: return 500
    def mileage_discount_rate(self) -> float: return 0.05
    def daily_insurance(self) -> float: return 10


class SUVRental(VehicleRental):
    def base_rate(self) -> float: return 50
    def per_mile_rate(self) -> float: return 0.20
    def mileage_threshold(self) -> float: return 300
    def mileage_discount_rate(self) -> float: return 0.08
    def daily_insurance(self) -> float: return 15


class VanRental(VehicleRental):
    def base_rate(self) -> float: return 45
    def per_mile_rate(self) -> float: return 0.18
    def mileage_threshold(self) -> float: return 400
    def mileage_discount_rate(self) -> float: return 0.06
    def daily_insurance(self) -> float: return 12


class MotorcycleRental(VehicleRental):
    def base_rate(self) -> float: return 20
    def per_mile_rate(self) -> float: return 0.10
    def mileage_threshold(self) -> float: return 200
    def mileage_discount_rate(self) -> float: return 0.03
    def daily_insurance(self) -> float: return 8


VEHICLE_MAP: dict[str, type[VehicleRental]] = {
    "SEDAN": SedanRental,
    "SUV": SUVRental,
    "VAN": VanRental,
    "MOTORCYCLE": MotorcycleRental,
}


def make_vehicle(vehicle_type: str) -> VehicleRental:
    cls = VEHICLE_MAP.get(vehicle_type)
    if cls is None:
        raise ValueError(f"Unknown vehicle type: {vehicle_type}")
    return cls()


def calculate_fee_v2(vehicle_type: str, days: int, miles: float, insurance: bool, discount: float) -> float:
    rental = Rental(days=days, miles=miles, insurance=insurance, discount=discount)
    vehicle = make_vehicle(vehicle_type)
    return vehicle.calculate_fee(rental)


def print_receipt_v2(vehicle_type: str, days: int, miles: float, insurance: bool, discount: float) -> None:
    print(f"=== Rental Receipt (V2: Polymorphism + Parameter Object) ===")
    print(f"Vehicle: {vehicle_type}")
    print(f"Days: {days}, Miles: {miles}")
    print(f"Insurance: {'Yes' if insurance else 'No'}")
    print(f"Discount: {discount * 100:.0f}%")
    print(f"Total: ${calculate_fee_v2(vehicle_type, days, miles, insurance, discount):.2f}")
    print()


# =============================================================================
# VERSION 3: REPLACE TEMP WITH QUERY + MOVE METHOD (final polished version)
# =============================================================================

class RentalCalculation:
    """Final refactored version: queries replace temps, logic lives with data."""

    def __init__(self, vehicle: VehicleRental, rental: Rental):
        self._vehicle = vehicle
        self._rental = rental

    @property
    def days(self) -> int:
        return self._rental.days

    @property
    def miles(self) -> float:
        return self._rental.miles

    @property
    def has_insurance(self) -> bool:
        return self._rental.insurance

    @property
    def discount_fraction(self) -> float:
        return self._rental.discount

    @property
    def base_charge(self) -> float:
        return self._vehicle.base_rate() * self.days

    @property
    def mileage_charge(self) -> float:
        return self._vehicle.calculate_mileage_charge(self._rental)

    @property
    def insurance_charge(self) -> float:
        return self._vehicle.daily_insurance() * self.days if self.has_insurance else 0.0

    @property
    def subtotal(self) -> float:
        return self.base_charge + self.mileage_charge + self.insurance_charge

    @property
    def total(self) -> float:
        return round(self.subtotal * (1 - self.discount_fraction), 2)


def print_receipt_final(vehicle_type: str, days: int, miles: float, insurance: bool, discount: float) -> None:
    vehicle = make_vehicle(vehicle_type)
    rental = Rental(days=days, miles=miles, insurance=insurance, discount=discount)
    calc = RentalCalculation(vehicle, rental)

    print(f"=== Rental Receipt (V3: Replace Temp with Query) ===")
    print(f"Vehicle: {vehicle_type}")
    print(f"Days: {calc.days}, Miles: {calc.miles}")
    print(f"Insurance: {'Yes' if calc.has_insurance else 'No'}")
    print(f"Discount: {calc.discount_fraction * 100:.0f}%")
    print(f"  Base charge:   ${calc.base_charge:.2f}")
    print(f"  Mileage:       ${calc.mileage_charge:.2f}")
    print(f"  Insurance:     ${calc.insurance_charge:.2f}")
    print(f"  Subtotal:      ${calc.subtotal:.2f}")
    print(f"  Total:         ${calc.total:.2f}")
    print()


# =============================================================================
# MAIN: Run all versions side by side to prove behavior is preserved
# =============================================================================

TEST_CASES: List[tuple] = [
    ("SEDAN", 3, 250, False, 0.0),
    ("SEDAN", 5, 600, True, 0.10),
    ("SUV", 2, 350, True, 0.05),
    ("VAN", 7, 500, False, 0.15),
    ("MOTORCYCLE", 1, 100, True, 0.0),
    ("MOTORCYCLE", 4, 250, True, 0.20),
]


def main() -> None:
    print("=" * 60)
    print("REFACTORING KATA: Before / After Comparison")
    print("=" * 60)
    print()

    print("--- V0: Smelly code (long method, feature envy, switch) ---")
    for vehicle_type, days, miles, insurance, discount in TEST_CASES:
        print_receipt_v0(vehicle_type, days, miles, insurance, discount)

    print("--- V1: After Extract Method + Extract Variable + Rename ---")
    for vehicle_type, days, miles, insurance, discount in TEST_CASES:
        print_receipt_v1(vehicle_type, days, miles, insurance, discount)

    print("--- V2: After Replace Conditional with Polymorphism + Parameter Object ---")
    for vehicle_type, days, miles, insurance, discount in TEST_CASES:
        print_receipt_v2(vehicle_type, days, miles, insurance, discount)

    print("--- V3: After Replace Temp with Query + Move Method (final) ---")
    for vehicle_type, days, miles, insurance, discount in TEST_CASES:
        print_receipt_final(vehicle_type, days, miles, insurance, discount)

    print("=" * 60)
    print("VERIFICATION: All versions produce identical totals")
    print("=" * 60)
    all_match = True
    for vehicle_type, days, miles, insurance, discount in TEST_CASES:
        v0 = calculate_fee_v0(vehicle_type, days, miles, insurance, discount)
        v1 = calculate_fee_v1(vehicle_type, days, miles, insurance, discount)
        v2 = calculate_fee_v2(vehicle_type, days, miles, insurance, discount)
        v3 = RentalCalculation(
            make_vehicle(vehicle_type),
            Rental(days=days, miles=miles, insurance=insurance, discount=discount)
        ).total
        match = v0 == v1 == v2 == v3
        status = "PASS" if match else "FAIL"
        print(f"  {vehicle_type:12s} d={days} m={miles:5.0f} ins={insurance!s:5s} disc={discount:.2f}  "
              f"v0={v0:7.2f} v1={v1:7.2f} v2={v2:7.2f} v3={v3:7.2f}  [{status}]")
        if not match:
            all_match = False

    print()
    if all_match:
        print("All versions produce identical output. Refactoring preserved behavior!")
    else:
        print("MISMATCH DETECTED — refactoring changed behavior!")


if __name__ == "__main__":
    main()