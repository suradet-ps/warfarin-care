export type Severity = 'contraindicated' | 'major' | 'moderate' | 'minor';

export interface Interaction {
  icode: string;
  drugName: string;
  strength?: string;
  interactionType: string;
  severity: Severity;
  clinicalEffect?: string;
  management?: string;
  evidenceLevel?: string;
}

export interface DrugInteraction {
  id: number;
  icode: string;
  drugName: string;
  strength?: string;
  interactionType: string;
  severity: string;
  clinicalEffect?: string;
  management?: string;
  evidenceLevel?: string;
  createdAt: string;
  updatedAt: string;
}

export interface DrugInteractionInput {
  icode: string;
  drugName: string;
  strength?: string;
  interactionType: string;
  severity: string;
  clinicalEffect?: string;
  management?: string;
  evidenceLevel?: string;
}
