module main

sig Service {
  deps: set Service
}

one sig Gateway extends Service {}

fact NoSelfDependency {
  all s: Service | s not in s.deps
}

fact Acyclic {
  no s: Service | s in s.^deps
}

fact ReachableFromGateway {
  all s: Service | s = Gateway or s in Gateway.^deps
}

assert NoCycles {
  no s: Service | s in s.^deps
}

check NoCycles for 6
run {} for 6
