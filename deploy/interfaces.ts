type AllDayEvent = string;
interface MultiDayEvent {
  from: string;
  to: string;
}
interface EventDateInfo {
  time?: string;
  date: AllDayEvent | MultiDayEvent;
}

export interface QuestionInfo {
  required: boolean;
  question: string;
}

export interface EventInfo {
  // Stage 1
  name: string;
  id: string;
  description: string;
  location: string;
  date: EventDateInfo;
  artwork: string;

  // Stage 2
  questions?: QuestionInfo[];
}

interface TicketInfo {
  name: string;
  eventId: string;
  description: string;
  salesValidThrough: string;
  passValidThrough: string;
  price: string;
  artwork: string;
  maxSupply?: number;
}

export interface DropMetadata {
  dateCreated: string;
  dropName: string;
  ticketInfo: TicketInfo;
  eventInfo?: EventInfo;
}

export interface Event {
  eventInfo: EventInfo;
  tickets: DropMetadata[];
}
