interface Formatter {
  format(input: string): string;
}

class UppercaseFormatter implements Formatter {
  format(input: string): string {
    return input.toUpperCase();
  }
}

class Service {
  private readonly formatter: Formatter;

  constructor(formatter: Formatter) {
    this.formatter = formatter;
  }
  run(s: string): string {
    return this.formatter.format(s);
  }
}

const svc = new Service(new UppercaseFormatter());
console.log(svc.run("composed behavior"));
