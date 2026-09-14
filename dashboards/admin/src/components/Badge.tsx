import { type ReactNode } from 'react';
import './Badge.css';

type Tone = 'neutral' | 'green' | 'yellow' | 'red' | 'blue';

interface Props {
  tone?: Tone;
  children: ReactNode;
}

export function Badge({ tone = 'neutral', children }: Props) {
  return <span className={`badge badge-${tone}`}>{children}</span>;
}
