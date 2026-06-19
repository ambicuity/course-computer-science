// Saga orchestrator — orchestration and choreography patterns with compensating transactions.
package main

import (
	"fmt"
	"log"
	"sync"
)

type SagaStep struct {
	Name         string
	Execute      func() (map[string]interface{}, error)
	Compensate  func(map[string]interface{}) error
}

type SagaResult struct {
	CompletedSteps []string
	StepOutputs    []map[string]interface{}
	FailedStep     string
	Compensated    []string
	Err            error
}

type SagaOrchestrator struct {
	Steps []SagaStep
}

func NewSagaOrchestrator(steps []SagaStep) *SagaOrchestrator {
	return &SagaOrchestrator{Steps: steps}
}

func (s *SagaOrchestrator) Run() SagaResult {
	var completedSteps []string
	var stepOutputs []map[string]interface{}

	for _, step := range s.Steps {
		output, err := step.Execute()
		if err != nil {
			log.Printf("SAGA: step %q failed: %v", step.Name, err)
			compensated := s.compensate(completedSteps, stepOutputs)
			return SagaResult{
				CompletedSteps: completedSteps,
				StepOutputs:    stepOutputs,
				FailedStep:      step.Name,
				Compensated:    compensated,
				Err:            err,
			}
		}
		log.Printf("SAGA: step %q succeeded", step.Name)
		completedSteps = append(completedSteps, step.Name)
		stepOutputs = append(stepOutputs, output)
	}

	return SagaResult{
		CompletedSteps: completedSteps,
		StepOutputs:    stepOutputs,
	}
}

func (s *SagaOrchestrator) compensate(completedSteps []string, stepOutputs []map[string]interface{}) []string {
	var compensated []string
	for i := len(completedSteps) - 1; i >= 0; i-- {
		stepName := completedSteps[i]
		output := stepOutputs[i]
		for _, step := range s.Steps {
			if step.Name == stepName {
				if cerr := step.Compensate(output); cerr != nil {
					log.Printf("SAGA: compensation for %q FAILED: %v", stepName, cerr)
				} else {
					log.Printf("SAGA: compensated %q", stepName)
				}
				compensated = append(compensated, stepName)
				break
			}
		}
	}
	return compensated
}

type Event struct {
	Type    string
	Payload map[string]interface{}
}

type EventBus struct {
	subscribers map[string][]func(Event)
	mu          sync.Mutex
}

func NewEventBus() *EventBus {
	return &EventBus{
		subscribers: make(map[string][]func(Event)),
	}
}

func (eb *EventBus) Subscribe(eventType string, handler func(Event)) {
	eb.mu.Lock()
	defer eb.mu.Unlock()
	eb.subscribers[eventType] = append(eb.subscribers[eventType], handler)
}

func (eb *EventBus) Publish(event Event) {
	eb.mu.Lock()
	handlers := make([]func(Event), len(eb.subscribers[event.Type]))
	copy(handlers, eb.subscribers[event.Type])
	eb.mu.Unlock()

	for _, h := range handlers {
		h(event)
	}
}

type ChoreographyService struct {
	Name       string
	OnEvent    func(Event) (map[string]interface{}, error)
	Compensate func(map[string]interface{}) error
	SuccessEventType string
	FailureEventType string
	bus        *EventBus
	state      map[string]interface{}
}

func (cs *ChoreographyService) Register(bus *EventBus) {
	cs.bus = bus
	bus.Subscribe(cs.Name+"_execute", func(e Event) {
		output, err := cs.OnEvent(e)
		if err != nil {
			bus.Publish(Event{Type: cs.FailureEventType, Payload: map[string]interface{}{"error": err.Error(), "from": cs.Name}})
			return
		}
		cs.state = output
		bus.Publish(Event{Type: cs.SuccessEventType, Payload: output})
	})
	bus.Subscribe(cs.Name+"_compensate", func(e Event) {
		if cs.state != nil {
			cs.Compensate(cs.state)
			cs.state = nil
		}
	})
}

var carReserved bool
var hotelBooked bool
var flightBooked bool

func resetState() {
	carReserved = false
	hotelBooked = false
	flightBooked = false
}

func bookTripSaga() {
	fmt.Println("=== ORCHESTRATED SAGA: Book a Trip ===")
	fmt.Println()

	steps := []SagaStep{
		{
			Name: "reserve_car",
			Execute: func() (map[string]interface{}, error) {
				carReserved = true
				return map[string]interface{}{"car_id": "CAR-123", "car_type": "sedan"}, nil
			},
			Compensate: func(output map[string]interface{}) error {
				carReserved = false
				log.Printf("  Cancelled car reservation: %v", output["car_id"])
				return nil
			},
		},
		{
			Name: "book_hotel",
			Execute: func() (map[string]interface{}, error) {
				hotelBooked = true
				return map[string]interface{}{"hotel_id": "HTL-456", "room": "deluxe"}, nil
			},
			Compensate: func(output map[string]interface{}) error {
				hotelBooked = false
				log.Printf("  Cancelled hotel booking: %v", output["hotel_id"])
				return nil
			},
		},
		{
			Name: "book_flight",
			Execute: func() (map[string]interface{}, error) {
				flightBooked = true
				return map[string]interface{}{"flight_id": "FLT-789", "seat": "12A"}, nil
			},
			Compensate: func(output map[string]interface{}) error {
				flightBooked = false
				log.Printf("  Cancelled flight booking: %v", output["flight_id"])
				return nil
			},
		},
	}

	orch := NewSagaOrchestrator(steps)
	result := orch.Run()

	fmt.Println()
	fmt.Printf("  Completed steps: %v\n", result.CompletedSteps)
	fmt.Printf("  Failed step:      %v\n", result.FailedStep)
	fmt.Printf("  Compensated:      %v\n", result.Compensated)
	fmt.Printf("  Final state:      car=%v hotel=%v flight=%v\n", carReserved, hotelBooked, flightBooked)
	fmt.Println()
}

func bookTripSagaWithFailure() {
	fmt.Println("=== ORCHESTRATED SAGA: Book a Trip (flight booking fails) ===")
	fmt.Println()

	steps := []SagaStep{
		{
			Name: "reserve_car",
			Execute: func() (map[string]interface{}, error) {
				carReserved = true
				return map[string]interface{}{"car_id": "CAR-123"}, nil
			},
			Compensate: func(output map[string]interface{}) error {
				carReserved = false
				log.Printf("  Cancelled car reservation: %v", output["car_id"])
				return nil
			},
		},
		{
			Name: "book_hotel",
			Execute: func() (map[string]interface{}, error) {
				hotelBooked = true
				return map[string]interface{}{"hotel_id": "HTL-456"}, nil
			},
			Compensate: func(output map[string]interface{}) error {
				hotelBooked = false
				log.Printf("  Cancelled hotel booking: %v", output["hotel_id"])
				return nil
			},
		},
		{
			Name: "book_flight",
			Execute: func() (map[string]interface{}, error) {
				return nil, fmt.Errorf("no seats available on requested flight")
			},
			Compensate: func(output map[string]interface{}) error {
				return nil
			},
		},
	}

	orch := NewSagaOrchestrator(steps)
	result := orch.Run()

	fmt.Println()
	fmt.Printf("  Completed steps: %v\n", result.CompletedSteps)
	fmt.Printf("  Failed step:      %v\n", result.FailedStep)
	fmt.Printf("  Compensated:      %v\n", result.Compensated)
	fmt.Printf("  Final state:      car=%v hotel=%v flight=%v\n", carReserved, hotelBooked, flightBooked)
	fmt.Printf("  Error:            %v\n", result.Err)
	fmt.Println()
}

func choreographySaga() {
	fmt.Println("=== CHOREOGRAPHED SAGA: Book a Trip ===")
	fmt.Println()

	resetState()

	bus := NewEventBus()

	var carOutput, hotelOutput map[string]interface{}

	carService := &ChoreographyService{
		Name:             "car",
		SuccessEventType: "car_reserved",
		FailureEventType: "saga_failed",
		OnEvent: func(e Event) (map[string]interface{}, error) {
			carReserved = true
			out := map[string]interface{}{"car_id": "CAR-CH-001"}
			carOutput = out
			fmt.Println("  [Car Service] Car reserved")
			return out, nil
		},
		Compensate: func(output map[string]interface{}) error {
			carReserved = false
			fmt.Println("  [Car Service] Car reservation cancelled")
			return nil
		},
	}

	hotelService := &ChoreographyService{
		Name:             "hotel",
		SuccessEventType: "hotel_booked",
		FailureEventType: "saga_failed",
		OnEvent: func(e Event) (map[string]interface{}, error) {
			hotelBooked = true
			out := map[string]interface{}{"hotel_id": "HTL-CH-002"}
			hotelOutput = out
			fmt.Println("  [Hotel Service] Hotel booked")
			return out, nil
		},
		Compensate: func(output map[string]interface{}) error {
			hotelBooked = false
			fmt.Println("  [Hotel Service] Hotel booking cancelled")
			return nil
		},
	}

	flightService := &ChoreographyService{
		Name:             "flight",
		SuccessEventType: "flight_booked",
		FailureEventType: "saga_failed",
		OnEvent: func(e Event) (map[string]interface{}, error) {
			return nil, fmt.Errorf("flight sold out")
		},
		Compensate: func(output map[string]interface{}) error {
			fmt.Println("  [Flight Service] No flight to cancel (was not booked)")
			return nil
		},
	}

	carService.Register(bus)
	hotelService.Register(bus)
	flightService.Register(bus)

	bus.Subscribe("car_reserved", func(e Event) {
		bus.Publish(Event{Type: "hotel_execute", Payload: e.Payload})
	})
	bus.Subscribe("hotel_booked", func(e Event) {
		bus.Publish(Event{Type: "flight_execute", Payload: e.Payload})
	})
	bus.Subscribe("saga_failed", func(e Event) {
		from := e.Payload["from"]
		fmt.Printf("  [Saga] Failure from %v — compensating...\n", from)
		bus.Publish(Event{Type: "flight_compensate", Payload: nil})
		bus.Publish(Event{Type: "hotel_compensate", Payload: hotelOutput})
		bus.Publish(Event{Type: "car_compensate", Payload: carOutput})
	})

	fmt.Println("Kicking off choreographed saga...")
	bus.Publish(Event{Type: "car_execute", Payload: nil})
	fmt.Println()
	fmt.Printf("  Final state:      car=%v hotel=%v flight=%v\n", carReserved, hotelBooked, flightBooked)
	fmt.Println()
}

func main() {
	resetState()
	bookTripSaga()

	resetState()
	bookTripSagaWithFailure()

	resetState()
	choreographySaga()
}