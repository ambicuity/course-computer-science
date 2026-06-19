package main

import (
	"fmt"
	"sync"
)

type Event struct {
	Type string
	Data map[string]interface{}
}

type EventHandler func(Event)

type EventBus struct {
	mu       sync.Mutex
	handlers map[string][]EventHandler
	processed map[string]bool
}

func NewEventBus() *EventBus {
	return &EventBus{
		handlers:  make(map[string][]EventHandler),
		processed: make(map[string]bool),
	}
}

func (bus *EventBus) Subscribe(eventType string, handler EventHandler) {
	bus.mu.Lock()
	defer bus.mu.Unlock()
	bus.handlers[eventType] = append(bus.handlers[eventType], handler)
}

func (bus *EventBus) Publish(event Event) {
	bus.mu.Lock()
	handlers := make([]EventHandler, len(bus.handlers[event.Type]))
	copy(handlers, bus.handlers[event.Type])
	bus.mu.Unlock()

	for _, handler := range handlers {
		handler(event)
	}
}

func (bus *EventBus) IsProcessed(id string) bool {
	bus.mu.Lock()
	defer bus.mu.Unlock()
	return bus.processed[id]
}

func (bus *EventBus) MarkProcessed(id string) {
	bus.mu.Lock()
	defer bus.mu.Unlock()
	bus.processed[id] = true
}

type InventoryService struct {
	bus      *EventBus
	reserved map[string]bool
	mu       sync.Mutex
}

func NewInventoryService(bus *EventBus) *InventoryService {
	inv := &InventoryService{
		bus:      bus,
		reserved: make(map[string]bool),
	}
	bus.Subscribe("OrderCreated", inv.handleOrderCreated)
	bus.Subscribe("PaymentFailed", inv.handlePaymentFailed)
	return inv
}

func (inv *InventoryService) handleOrderCreated(event Event) {
	orderID := event.Data["orderId"].(string)
	if inv.bus.IsProcessed("reserve-" + orderID) {
		fmt.Printf("  [Inventory] Skipping duplicate reservation for order %s\n", orderID)
		return
	}
	inv.mu.Lock()
	inv.reserved[orderID] = true
	inv.mu.Unlock()
	inv.bus.MarkProcessed("reserve-" + orderID)
	fmt.Printf("  [Inventory] Reserved stock for order %s\n", orderID)
	inv.bus.Publish(Event{
		Type: "InventoryReserved",
		Data: map[string]interface{}{"orderId": orderID},
	})
}

func (inv *InventoryService) handlePaymentFailed(event Event) {
	orderID := event.Data["orderId"].(string)
	inv.mu.Lock()
	if inv.reserved[orderID] {
		delete(inv.reserved, orderID)
		inv.mu.Unlock()
		fmt.Printf("  [Inventory] Released stock for order %s (compensating)\n", orderID)
		inv.bus.Publish(Event{
			Type: "InventoryReleased",
			Data: map[string]interface{}{"orderId": orderID},
		})
		return
	}
	inv.mu.Unlock()
	fmt.Printf("  [Inventory] No reservation to release for order %s\n", orderID)
}

type PaymentService struct {
	bus       *EventBus
	charged   map[string]bool
	mu        sync.Mutex
	shouldFail bool
}

func NewPaymentService(bus *EventBus, shouldFail bool) *PaymentService {
	pay := &PaymentService{
		bus:        bus,
		charged:    make(map[string]bool),
		shouldFail: shouldFail,
	}
	bus.Subscribe("InventoryReserved", pay.handleInventoryReserved)
	return pay
}

func (pay *PaymentService) handleInventoryReserved(event Event) {
	orderID := event.Data["orderId"].(string)
	if pay.bus.IsProcessed("pay-" + orderID) {
		fmt.Printf("  [Payment] Skipping duplicate charge for order %s\n", orderID)
		return
	}
	pay.mu.Lock()
	pay.charged[orderID] = true
	pay.mu.Unlock()
	pay.bus.MarkProcessed("pay-" + orderID)

	if pay.shouldFail {
		fmt.Printf("  [Payment] FAILED to charge order %s\n", orderID)
		pay.bus.Publish(Event{
			Type: "PaymentFailed",
			Data: map[string]interface{}{"orderId": orderID},
		})
		return
	}

	fmt.Printf("  [Payment] Charged order %s\n", orderID)
	pay.bus.Publish(Event{
		Type: "PaymentProcessed",
		Data: map[string]interface{}{"orderId": orderID},
	})
}

type ShippingService struct {
	bus    *EventBus
	shipped map[string]bool
	mu     sync.Mutex
}

func NewShippingService(bus *EventBus) *ShippingService {
	ship := &ShippingService{
		bus:     bus,
		shipped: make(map[string]bool),
	}
	bus.Subscribe("PaymentProcessed", ship.handlePaymentProcessed)
	return ship
}

func (ship *ShippingService) handlePaymentProcessed(event Event) {
	orderID := event.Data["orderId"].(string)
	if ship.bus.IsProcessed("ship-" + orderID) {
		fmt.Printf("  [Shipping] Skipping duplicate shipment for order %s\n", orderID)
		return
	}
	ship.mu.Lock()
	ship.shipped[orderID] = true
	ship.mu.Unlock()
	ship.bus.MarkProcessed("ship-" + orderID)
	fmt.Printf("  [Shipping] Shipped order %s\n", orderID)
	ship.bus.Publish(Event{
		Type: "OrderShipped",
		Data: map[string]interface{}{"orderId": orderID},
	})
}

type OrderService struct {
	bus *EventBus
}

func NewOrderService(bus *EventBus) *OrderService {
	return &OrderService{bus: bus}
}

func (o *OrderService) CreateOrder(orderID string) {
	fmt.Printf("[Order] Creating order %s\n", orderID)
	o.bus.Publish(Event{
		Type: "OrderCreated",
		Data: map[string]interface{}{"orderId": orderID},
	})
}

type AnalyticsService struct {
	bus     *EventBus
	events  []string
	mu      sync.Mutex
}

func NewAnalyticsService(bus *EventBus) *AnalyticsService {
	a := &AnalyticsService{bus: bus}
	bus.Subscribe("OrderCreated", a.trackEvent)
	bus.Subscribe("OrderShipped", a.trackEvent)
	bus.Subscribe("PaymentFailed", a.trackEvent)
	return a
}

func (a *AnalyticsService) trackEvent(event Event) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.events = append(a.events, event.Type)
}

func (a *AnalyticsService) PrintLog() {
	a.mu.Lock()
	defer a.mu.Unlock()
	fmt.Printf("\n[Analytics] Tracked events: %v\n", a.events)
}

func demoSuccessfulOrder() {
	fmt.Println("=== Demo 1: Successful Order Saga ===")
	fmt.Println("Flow: OrderCreated → InventoryReserved → PaymentProcessed → OrderShipped")
	fmt.Println()

	bus := NewEventBus()
	_ = NewAnalyticsService(bus)
	_ = NewInventoryService(bus)
	_ = NewPaymentService(bus, false)
	_ = NewShippingService(bus)
	orderSvc := NewOrderService(bus)

	orderSvc.CreateOrder("ORD-001")

	fmt.Println("\n  ✅ Order completed successfully via choreographed saga")
}

func demoFailedPayment() {
	fmt.Println("\n\n=== Demo 2: Failed Payment — Saga Compensation ===")
	fmt.Println("Flow: OrderCreated → InventoryReserved → PaymentFailed → InventoryReleased")
	fmt.Println()

	bus := NewEventBus()
	_ = NewAnalyticsService(bus)
	_ = NewInventoryService(bus)
	_ = NewPaymentService(bus, true)
	_ = NewShippingService(bus)
	orderSvc := NewOrderService(bus)

	orderSvc.CreateOrder("ORD-002")

	fmt.Println("\n  ❌ Order failed, compensating transactions executed")
}

func demoIdempotency() {
	fmt.Println("\n\n=== Demo 3: Idempotent Handlers — Duplicate Event ===")
	fmt.Println("Publishing OrderCreated twice for the same order")
	fmt.Println()

	bus := NewEventBus()
	_ = NewInventoryService(bus)
	_ = NewPaymentService(bus, false)
	_ = NewShippingService(bus)
	_ = NewOrderService(bus)

	fmt.Println("First event:")
	bus.Publish(Event{
		Type: "OrderCreated",
		Data: map[string]interface{}{"orderId": "ORD-003"},
	})
	fmt.Println("\nSecond event (duplicate):")
	bus.Publish(Event{
		Type: "OrderCreated",
		Data: map[string]interface{}{"orderId": "ORD-003"},
	})
	fmt.Println("\n  🔄 Second event was skipped by idempotent handlers")
}

func demoDecoupling() {
	fmt.Println("\n\n=== Demo 4: Adding a Consumer Without Touching Producers ===")
	fmt.Println("New AuditService subscribes to existing events—no producer changes needed")
	fmt.Println()

	bus := NewEventBus()
	auditEvents := []string{}
	auditHandler := func(event Event) {
		auditEvents = append(auditEvents, event.Type)
		fmt.Printf("  [Audit] Logged event: %s\n", event.Type)
	}
	bus.Subscribe("OrderCreated", auditHandler)
	bus.Subscribe("OrderShipped", auditHandler)

	_ = NewInventoryService(bus)
	_ = NewPaymentService(bus, false)
	_ = NewShippingService(bus)
	_ = NewOrderService(bus)

	bus.Publish(Event{
		Type: "OrderCreated",
		Data: map[string]interface{}{"orderId": "ORD-004"},
	})

	fmt.Printf("\n  📋 Audit log captured %d events without any producer knowing about it\n", len(auditEvents))
}

func main() {
	demoSuccessfulOrder()
	demoFailedPayment()
	demoIdempotency()
	demoDecoupling()
}