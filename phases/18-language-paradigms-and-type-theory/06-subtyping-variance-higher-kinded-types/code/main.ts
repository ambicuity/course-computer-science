type Animal = { name: string };
type Dog = Animal & { bark: () => void };

type Producer<T> = () => T;
type Consumer<T> = (v: T) => void;

const dogProducer: Producer<Dog> = () => ({ name: "pup", bark: () => {} });
const animalProducer: Producer<Animal> = dogProducer; // covariance-like

const animalConsumer: Consumer<Animal> = (a) => console.log(a.name);
const dogConsumer: Consumer<Dog> = animalConsumer; // contravariance-like under function parameter checks

const d = animalProducer();
dogConsumer({ ...d, bark: () => {} });
console.log("variance demo complete");
