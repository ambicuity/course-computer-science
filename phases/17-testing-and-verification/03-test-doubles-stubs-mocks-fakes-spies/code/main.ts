type User = {
  userId: string;
  preferredChannel: "email" | "sms";
};

class PreferenceStub {
  constructor(private readonly mapping: Record<string, "email" | "sms">) {}

  getChannel(userId: string): "email" | "sms" {
    return this.mapping[userId] ?? "email";
  }
}

class AuditFake {
  public rows: string[] = [];

  save(record: string): void {
    this.rows.push(record);
  }
}

class SenderSpy {
  public sentPayloads: string[] = [];

  send(channel: string, payload: string): void {
    this.sentPayloads.push(`${channel}:${payload}`);
  }
}

class StrictSenderMock {
  public calls = 0;

  constructor(private readonly expectedCorrelation: string) {}

  send(_channel: string, payload: string): void {
    if (!payload.includes(this.expectedCorrelation)) {
      throw new Error("missing required correlation ID");
    }
    this.calls += 1;
  }
}

class NotificationService {
  constructor(
    private readonly prefRepo: PreferenceStub,
    private readonly sender: { send: (channel: string, payload: string) => void },
    private readonly auditRepo: AuditFake,
  ) {}

  notify(user: User, message: string, correlationId: string): void {
    const channel = this.prefRepo.getChannel(user.userId);
    const payload = `cid=${correlationId};msg=${message}`;
    this.sender.send(channel, payload);
    this.auditRepo.save(`${user.userId}|${channel}|${payload}`);
  }
}

function assert(condition: boolean, message: string): void {
  if (!condition) throw new Error(message);
}

function runDemo(): void {
  const prefs = new PreferenceStub({ "u-1": "sms" });
  const spy = new SenderSpy();
  const audit = new AuditFake();
  const service = new NotificationService(prefs, spy, audit);

  service.notify({ userId: "u-1", preferredChannel: "sms" }, "deploy ok", "req-1");
  assert(spy.sentPayloads.length === 1, "expected one send");
  assert(audit.rows.length === 1, "expected one audit row");

  const strict = new StrictSenderMock("cid=req-2");
  const service2 = new NotificationService(new PreferenceStub({}), strict, new AuditFake());
  service2.notify({ userId: "u-2", preferredChannel: "email" }, "invoice", "req-2");
  assert(strict.calls === 1, "expected one strict mock call");
}

runDemo();
console.log("test doubles demo passed");
