export type OutcomeType =
  | 'major_bleeding'
  | 'minor_bleeding'
  | 'thromboembolism'
  | 'hospitalization'
  | 'death'
  | 'other';

export interface WfOutcome {
  id: number;
  hn: string;
  eventDate: string;
  eventType: OutcomeType;
  description?: string;
  inrAtEvent?: number;
  actionTaken?: string;
  createdBy?: string;
  createdAt: string;
}

export interface OutcomeInput {
  hn: string;
  eventDate: string;
  eventType: OutcomeType;
  description?: string;
  inrAtEvent?: number;
  actionTaken?: string;
  createdBy?: string;
}
