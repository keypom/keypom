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

export interface DropMetadata {
  dateCreated: string;
  name: string;
  eventId: string; // UUID
  description: string;
  salesValidThrough: string;
  passValidThrough: string;
  price: string;
  artwork: string;
  maxSupply?: number;
}

/// Maps UUID to Event Metadata
export type FunderMetadata = Record<string, FunderEventMetadata>;

export interface FunderEventMetadata {
  name: string;
  description: string;
  location: string;
  date: EventDateInfo;
  artwork: string;
  id: string; // UUID
  dateCreated: string;

  // Stage 2
  questions?: QuestionInfo[];

  // If there are some questions, then we need to encrypt the answers
  pubKey?: string;
  encPrivKey?: string;
  iv?: string;
  salt?: string;
}

export interface ZombieReturnedEvent {
  eventMeta: FunderEventMetadata;
  tickets: DropMetadata[];
}
