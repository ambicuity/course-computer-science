//! Modern Patterns — Functional Core / Imperative Shell
//! Phase 16 — Software Engineering & Architecture
//!
//! Demonstrates separating pure business logic (functional core) from
//! side effects (imperative shell) in an order processing system.

use std::fmt;

// ============================================================================
// FUNCTIONAL CORE — Pure functions, no side effects, no dependencies
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
struct Order {
    customer_id: String,
    items: Vec<OrderItem>,
}

#[derive(Debug, Clone, PartialEq)]
struct OrderItem {
    product_id: String,
    name: String,
    quantity: u32,
    unit_price: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct ValidatedOrder {
    order: Order,
}

#[derive(Debug, Clone, PartialEq)]
struct PricedOrder {
    order: Order,
    subtotal: f64,
    discount_amount: f64,
    total: f64,
    discount_reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CustomerTier {
    Gold,
    Silver,
    Bronze,
}

#[derive(Debug, Clone, PartialEq)]
struct Discount {
    amount: f64,
    reason: String,
}

impl Discount {
    fn new(amount: f64, reason: &str) -> Self {
        Discount {
            amount,
            reason: reason.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ValidationError {
    EmptyOrder,
    MissingCustomerId,
    NegativeQuantity { product_id: String },
}

#[derive(Debug, Clone, PartialEq)]
enum OrderDecision {
    Accept(PricedOrder),
    Reject(ValidationError),
}

impl Order {
    fn subtotal(&self) -> f64 {
        self.items.iter().map(|i| i.quantity as f64 * i.unit_price).sum()
    }
}

fn validate_order(order: &Order) -> Result<ValidatedOrder, ValidationError> {
    if order.customer_id.is_empty() {
        return Err(ValidationError::MissingCustomerId);
    }
    if order.items.is_empty() {
        return Err(ValidationError::EmptyOrder);
    }
    for item in &order.items {
        if item.quantity == 0 {
            return Err(ValidationError::NegativeQuantity {
                product_id: item.product_id.clone(),
            });
        }
    }
    Ok(ValidatedOrder {
        order: order.clone(),
    })
}

fn calculate_discount(order: &ValidatedOrder, tier: CustomerTier) -> Discount {
    let base = order.order.subtotal();
    match tier {
        CustomerTier::Gold => Discount::new(base * 0.15, "gold_tier"),
        CustomerTier::Silver => Discount::new(base * 0.10, "silver_tier"),
        CustomerTier::Bronze => Discount::new(base * 0.05, "bronze_tier"),
    }
}

fn apply_discount(order: &ValidatedOrder, discount: &Discount) -> PricedOrder {
    let subtotal = order.order.subtotal();
    let total = (subtotal - discount.amount).max(0.0);
    PricedOrder {
        order: order.order.clone(),
        subtotal,
        discount_amount: discount.amount,
        total,
        discount_reason: discount.reason.clone(),
    }
}

fn process_order_core(order: &Order, tier: CustomerTier) -> OrderDecision {
    match validate_order(order) {
        Ok(validated) => {
            let discount = calculate_discount(&validated, tier);
            let priced = apply_discount(&validated, &discount);
            OrderDecision::Accept(priced)
        }
        Err(e) => OrderDecision::Reject(e),
    }
}

// ============================================================================
// IMPERATIVE SHELL — I/O, database, HTTP, logging
// ============================================================================

struct ShellConfig {
    db_url: String,
    log_level: String,
}

fn shell_fetch_customer_tier(customer_id: &str, config: &ShellConfig) -> CustomerTier {
    println!("[SHELL][db] Querying tier for customer '{}' at {}", customer_id, config.db_url);
    if customer_id.starts_with("G") {
        CustomerTier::Gold
    } else if customer_id.starts_with("S") {
        CustomerTier::Silver
    } else {
        CustomerTier::Bronze
    }
}

fn shell_save_order(order: &PricedOrder, config: &ShellConfig) {
    println!(
        "[SHELL][db] Saving order for customer '{}' — total: ${:.2} at {}",
        order.order.customer_id, order.total, config.db_url
    );
}

fn shell_send_notification(order: &PricedOrder) {
    println!(
        "[SHELL][http] POST /notify — customer {} charged ${:.2} (discount: ${:.2} — {})",
        order.order.customer_id, order.total, order.discount_amount, order.discount_reason
    );
}

fn shell_log_rejection(error: &ValidationError) {
    println!("[SHELL][log] Order rejected: {:?}", error);
}

fn shell_run(order: &Order, config: &ShellConfig) {
    println!("[SHELL] --- Processing order for customer '{}' ---", order.customer_id);

    let tier = shell_fetch_customer_tier(&order.customer_id, config);

    let decision = process_order_core(order, tier);

    match decision {
        OrderDecision::Accept(priced) => {
            shell_save_order(&priced, config);
            shell_send_notification(&priced);
            println!(
                "[SHELL] ✓ Order accepted — subtotal: ${:.2}, discount: ${:.2}, total: ${:.2}",
                priced.subtotal, priced.discount_amount, priced.total
            );
        }
        OrderDecision::Reject(ref err) => {
            shell_log_rejection(err);
            println!("[SHELL] ✗ Order rejected");
        }
    }
}

// ============================================================================
// TESTS — Pure core tests require NO infrastructure
// ============================================================================

#[cfg(test)]
mod core_tests {
    use super::*;

    fn sample_order() -> Order {
        Order {
            customer_id: "CUST-001".to_string(),
            items: vec![
                OrderItem {
                    product_id: "SKU-A".to_string(),
                    name: "Widget".to_string(),
                    quantity: 3,
                    unit_price: 10.0,
                },
                OrderItem {
                    product_id: "SKU-B".to_string(),
                    name: "Gadget".to_string(),
                    quantity: 1,
                    unit_price: 25.0,
                },
            ],
        }
    }

    #[test]
    fn test_validate_valid_order() {
        let order = sample_order();
        let result = validate_order(&order);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().order, order);
    }

    #[test]
    fn test_validate_empty_customer_id() {
        let mut order = sample_order();
        order.customer_id = String::new();
        assert_eq!(validate_order(&order), Err(ValidationError::MissingCustomerId));
    }

    #[test]
    fn test_validate_empty_order() {
        let order = Order {
            customer_id: "CUST-001".to_string(),
            items: vec![],
        };
        assert_eq!(validate_order(&order), Err(ValidationError::EmptyOrder));
    }

    #[test]
    fn test_calculate_discount_gold() {
        let order = sample_order();
        let validated = validate_order(&order).unwrap();
        let discount = calculate_discount(&validated, CustomerTier::Gold);
        assert!((discount.amount - 8.25).abs() < 0.001);
        assert_eq!(discount.reason, "gold_tier");
    }

    #[test]
    fn test_calculate_discount_silver() {
        let order = sample_order();
        let validated = validate_order(&order).unwrap();
        let discount = calculate_discount(&validated, CustomerTier::Silver);
        assert!((discount.amount - 5.50).abs() < 0.001);
    }

    #[test]
    fn test_apply_discount_total_never_negative() {
        let order = sample_order();
        let validated = validate_order(&order).unwrap();
        let discount = Discount::new(99999.0, "mega_sale");
        let priced = apply_discount(&validated, &discount);
        assert!((priced.total - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_process_order_accept() {
        let order = sample_order();
        let decision = process_order_core(&order, CustomerTier::Gold);
        match decision {
            OrderDecision::Accept(priced) => {
                assert!((priced.total - 46.75).abs() < 0.01);
            }
            OrderDecision::Reject(_) => panic!("Expected Accept"),
        }
    }

    #[test]
    fn test_process_order_reject_empty() {
        let order = Order {
            customer_id: "CUST-001".to_string(),
            items: vec![],
        };
        let decision = process_order_core(&order, CustomerTier::Gold);
        assert!(matches!(decision, OrderDecision::Reject(ValidationError::EmptyOrder)));
    }
}

// ============================================================================
// MAIN — Shell orchestrates the whole flow
// ============================================================================

fn main() {
    let config = ShellConfig {
        db_url: "postgres://localhost/orders".to_string(),
        log_level: "info".to_string(),
    };

    let order = Order {
        customer_id: "GOLD-CUST-42".to_string(),
        items: vec![
            OrderItem {
                product_id: "SKU-WIDGET".to_string(),
                name: "Premium Widget".to_string(),
                quantity: 5,
                unit_price: 20.0,
            },
            OrderItem {
                product_id: "SKU-GADGET".to_string(),
                name: "Super Gadget".to_string(),
                quantity: 2,
                unit_price: 50.0,
            },
        ],
    };

    shell_run(&order, &config);

    println!();

    let bad_order = Order {
        customer_id: String::new(),
        items: vec![],
    };
    shell_run(&bad_order, &config);
}