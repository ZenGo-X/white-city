---------------------------- MODULE RelayServer ----------------------------

EXTENDS Integers

CONSTANT 
  PARTIES,  \* The set of parties, i.e p1,p2,p3
  ROUNDS     \* The set of rounds, i.e 1,2,3,4

ASSUME ROUNDS \subseteq Nat

VARIABLES
  partyState,         \* partyState[p] is the state of party r.
  serverState,        \* The state of the server.
  readyParties,       \* The set of parties that signal they are ready    
  assignedParties,    \* The set of parties that the server assigned them ID
  msgs                
    \* In the protocol, processes communicate with one another by sending  
    \* messages.  For simplicity, we represent message passing with the   
    \* variable msgs whose value is the set of all messages that have been 
    \* sent.  A message is sent by adding it to the set msgs.  An action   
    \* that, in an implementation, would be enabled by the receipt of a    
    \* certain message is here enabled by the presence of that message in  
    \* msgs.  For simplicity, messages are never removed from msgs.  This  
    \* allows a single message to be received by multiple receivers. 
-----------------------------------------------------------------------------

Messages ==
  [type : {"Abort","Start"}]
  \cup [type : {"Ready"}, party : PARTIES]
  \cup [type : {"Assign"}, party : PARTIES]
  \cup [type : {"AbortReq"}, party : PARTIES]
  
  \cup [type: {"P2P"}, from: PARTIES ,to: PARTIES, round: ROUNDS \ {0}]
  \cup [type: {"RelayP2P"}, from: PARTIES ,to: PARTIES, round: ROUNDS \ {0}]
  \cup [type: {"Broadcast"}, party: PARTIES, round: ROUNDS \ {0}]
  \cup [type: {"RelayBroadcast"}, party: PARTIES, round: ROUNDS \ {0}]

-----------------------------------------------------------------------------

\* The type-correctness invariant
TypeOK ==  
  /\ partyState \in [PARTIES -> {"idle", "ready", "assigned", "aborted"}]
  /\ serverState \in {"init", "running"}
  /\ readyParties \subseteq PARTIES
  /\ assignedParties \subseteq PARTIES
  /\ msgs \subseteq Messages
-----------------------------------------------------------------------------

\* The initial predicate. 
Init ==   
  /\ partyState = [p \in PARTIES |-> "idle"]
  /\ serverState = "init"
  /\ readyParties   = {}
  /\ assignedParties   = {}
  /\ msgs = {}
-----------------------------------------------------------------------------

\* At first, joining parties must signal the server
PartyReady(p) ==
    /\ serverState = "init"
    /\ msgs' = msgs \cup {[type |-> "Ready", party |-> p]}
    /\ p \notin readyParties
    /\ readyParties' = readyParties \cup {p}
    /\ partyState' = [partyState EXCEPT ![p] = "ready"] 
    /\ UNCHANGED <<serverState, assignedParties>>
    
\* The server will assign and send ID to each party that sent a signal ready
Assign(p) ==
    /\ serverState = "init"
    /\ [type |-> "Ready", party |-> p] \in msgs
    /\ [type |-> "Assign", party |-> p] \notin msgs
    /\ msgs' = msgs \cup {[type |-> "Assign", party |-> p]}
    /\ assignedParties' = assignedParties \cup {p}
    /\ partyState' = [partyState EXCEPT ![p] = "assigned"] 
    /\ UNCHANGED <<serverState, readyParties>>
  
\* Once |assigned parties| == N where N (PARTIES) 
\* is the constant the server know that represents
\* the number of parties in the protocol then the 
\* server changes his state and starts the protocol
Start ==
  /\ serverState = "init"
  /\ assignedParties = PARTIES
  /\ serverState' = "running"
  /\ msgs' = msgs \cup {[type |-> "Start"]}
  /\ UNCHANGED <<partyState, readyParties, assignedParties>>

\* When the protocol runs each party can send abort to the server
PartyAbort(p) == 
  /\ partyState[p] = "assigned"
  /\ partyState' = [partyState EXCEPT ![p] = "aborted"]
  /\ [type |-> "AbortReq", party |-> p] \in msgs
  /\ serverState = "running"
  /\ serverState' = "init"
  /\ msgs' = {[type |-> "Abort"]}
  /\ UNCHANGED <<readyParties>>
  
  
 \* The server upon receiving abort will stop the protocol and return all state to INIT
Abort ==
  /\ serverState = "init"
  /\ [type |-> "Abort"] \in msgs
  /\ readyParties'   = {}
  /\ msgs' = {}
  /\ partyState' = [p \in PARTIES |-> "idle"]
  /\ UNCHANGED <<serverState>>
  

\* When the protocol runs each party can ask the server once every round 
\* to relay a broadcast message. Notice there is no enforcing of the order 
\* of rounds  
ReqToBroadcast(r,p) == 
  /\ assignedParties = PARTIES
  /\ serverState = "running"
  /\ [type |-> "Broadcast", party |-> p, round |-> r] \notin msgs
  /\ msgs' = msgs \cup {[type |-> "Broadcast", party |-> p, round |-> r]}
  /\ UNCHANGED <<serverState,partyState,readyParties,assignedParties>>

\* The server upon receiving a reqest to broadcast will relay the message 
\* to all parties. Notice: the sending party might get her message as well
RelayBroadcast(r,p) == 
  /\ assignedParties = PARTIES
  /\ serverState = "running"
  /\ [type |-> "Broadcast", party |-> p, round |-> r] \in msgs
  /\ [type |-> "RelayBroadcast", party |-> p, round |-> r] \notin msgs
  /\ msgs' = msgs \cup {[type |-> "RelayBroadcast", party |-> p, round |-> r]}
  /\ UNCHANGED <<serverState,partyState,readyParties,assignedParties>>

\* When the protocol runs each party can ask the server once every round 
\* to send a message to another party. Notice the receiver can also be the sender
ReqToP2P(r,p1,p2) == 
  /\ assignedParties = PARTIES
  /\ serverState = "running"
  /\ [type |-> "P2P", from |-> p1, to |-> p2, round |-> r] \notin msgs
  /\ msgs' = msgs \cup {[type |-> "P2P",from |-> p1, to |-> p2, round |-> r]}
  /\ UNCHANGED <<serverState,partyState,readyParties,assignedParties>>

\* The server will relay p2p messages
RelayP2P(r,p1,p2) == 
  /\ assignedParties = PARTIES
  /\ serverState = "running"
  /\ [type |-> "P2P",from |-> p1, to |-> p2, round |-> r] \in msgs
  /\ [type |-> "RelayP2P",from |-> p1, to |-> p2, round |-> r] \notin msgs
  /\ msgs' = msgs \cup {[type |-> "RelayP2P", from |-> p1, to |-> p2, round |-> r]}
  /\ UNCHANGED <<serverState,partyState,readyParties,assignedParties>>
        

-----------------------------------------------------------------------------
Next ==
  Start \/ Abort
  \/ (\E p \in PARTIES :  PartyAbort(p) )
  \/ (\E p \in PARTIES :  PartyReady(p) )
  \/ (\E p \in PARTIES :  Assign(p) )
  \/ (\E p \in PARTIES : \E r \in ROUNDS : ReqToBroadcast(r,p))
  \/ (\E p \in PARTIES : \E r \in ROUNDS : RelayBroadcast(r,p) )
  \/ (\E p1 \in PARTIES : \E p2 \in PARTIES :  \E r \in ROUNDS : ReqToP2P(r,p1,p2))
  \/ (\E p1 \in PARTIES : \E p2 \in PARTIES :  \E r \in ROUNDS : RelayP2P(r,p1,p2))

Spec == Init /\ [][Next]_<<partyState, serverState, readyParties, assignedParties, msgs>>
THEOREM Spec => []TypeOK
=============================================================================
\* Modification History
\* Last modified Mon Dec 10 21:24:30 IST 2018 by omershlo
\* Created Mon Dec 10 21:18:59 IST 2018 by omershlo
