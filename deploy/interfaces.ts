export interface QuestionInfo {
  required: boolean;
  question: string;
}

export interface EventDropMetadata extends TicketInfoMetadata {
  dropName: string;
  eventId: string;
}

export interface DateAndTimeInfo {
  startDate: number; // Milliseconds from Unix Epoch

  startTime?: string; // Raw time string such as 9:00 AM
  // For single day events, toDay is not required
  endDate?: number; // Milliseconds from Unix Epoch
  endTime?: string; // Raw time string such as 9:00 AM
}

export interface TicketMetadataExtra {
  eventId: string;
  dateCreated: string;
  salesValidThrough: DateAndTimeInfo;
  passValidThrough: DateAndTimeInfo;
  price: string;
  maxSupply?: number;
}

export interface TicketInfoMetadata {
  title: string;
  description: string;
  media: string; // CID to IPFS. To render, use `${CLOUDFLARE_IPDS}/${media}`
  extra: string; // Stringified TicketMetadataExtra
}

/// Maps UUID to Event Metadata
export type FunderMetadata = Record<string, FunderEventMetadata>;

export interface FunderEventMetadata {
  // Stage 1
  name: string;
  id: string;
  description: string;
  location: string;
  date: DateAndTimeInfo;
  artwork: string;
  dateCreated: string;

  // Stage 2
  questions?: QuestionInfo[];

  // If there are some questions, then we need to encrypt the answers
  pubKey?: string;
  encPrivKey?: string;
  iv?: string;
  salt?: string;
}

export interface ZombieDropMetadata {
  dateCreated: string;
  name: string;
  eventId: string; // UUID
  description: string;
  salesValidThrough: DateAndTimeInfo;
  passValidThrough: DateAndTimeInfo;
  price: string;
  artwork: string;
  maxSupply?: number;
}

export interface ZombieReturnedEvent {
  eventMeta: FunderEventMetadata;
  tickets: ZombieDropMetadata[];
}
