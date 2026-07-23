export interface InrRecord {
  date: string;
  value: number;
  source: 'lab_order' | 'lab_app_order' | 'manual';
  labOrderNumber?: string;
  vn?: string;
}
