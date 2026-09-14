import { type InputHTMLAttributes, type ReactNode, forwardRef } from 'react';
import './Input.css';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  hint?: ReactNode;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { label, error, hint, className = '', id, ...rest },
  ref,
) {
  const inputId = id || rest.name;
  return (
    <div className="field">
      {label ? (
        <label htmlFor={inputId} className="field-label">
          {label}
        </label>
      ) : null}
      <input
        ref={ref}
        id={inputId}
        className={`input ${error ? 'input-error' : ''} ${className}`}
        {...rest}
      />
      {hint ? <span className="field-hint">{hint}</span> : null}
      {error ? <span className="field-error">{error}</span> : null}
    </div>
  );
});
