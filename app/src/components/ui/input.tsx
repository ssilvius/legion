/**
 * Form input component with validation states and accessibility
 *
 * @cognitive-load 4/10 - Data entry with validation feedback requires user attention
 * @attention-economics State hierarchy: default=ready, focus=active input, error=requires attention, success=validation passed
 * @trust-building Clear validation feedback, error recovery patterns, progressive enhancement
 * @accessibility Screen reader labels, validation announcements, keyboard navigation, high contrast support
 * @semantic-meaning Type-appropriate validation: email=format validation, password=security indicators, number=range constraints
 *
 * @usage-patterns
 * DO: Always pair with descriptive Label component
 * DO: Use helpful placeholders showing format examples
 * DO: Provide real-time validation for user confidence
 * DO: Use appropriate input types for sensitive data
 * NEVER: Label-less inputs, validation only on submit, unclear error messages
 *
 * @example
 * ```tsx
 * // Basic input with label
 * <Label htmlFor="email">Email</Label>
 * <Input id="email" type="email" placeholder="you@example.com" />
 *
 * // Error state
 * <Input variant="error" placeholder="Invalid input" />
 *
 * // Success state
 * <Input variant="success" defaultValue="Valid input" />
 *
 * // Sizes
 * <Input size="sm" placeholder="Small" />
 * <Input size="lg" placeholder="Large" />
 * ```
 */
import * as React from 'react';
import classy from '@/src/lib/primitives/classy';

export interface InputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size'> {
  variant?:
    | 'default'
    | 'primary'
    | 'secondary'
    | 'destructive'
    | 'success'
    | 'warning'
    | 'info'
    | 'muted'
    | 'accent';
  /** Size variant (not the HTML size attribute) */
  size?: 'sm' | 'default' | 'lg';
}

// Variant classes per docs/COMPONENT_STYLING_REFERENCE.md
const variantClasses: Record<string, string> = {
  default: 'border-primary focus-visible:ring-2 focus-visible:ring-primary-ring',
  primary: 'border-primary focus-visible:ring-2 focus-visible:ring-primary-ring',
  secondary: 'border-secondary focus-visible:ring-2 focus-visible:ring-secondary-ring',
  destructive: 'border-destructive focus-visible:ring-2 focus-visible:ring-destructive-ring',
  success: 'border-success focus-visible:ring-2 focus-visible:ring-success-ring',
  warning: 'border-warning focus-visible:ring-2 focus-visible:ring-warning-ring',
  info: 'border-info focus-visible:ring-2 focus-visible:ring-info-ring',
  muted: 'border-muted focus-visible:ring-2 focus-visible:ring-ring',
  accent: 'border-accent focus-visible:ring-2 focus-visible:ring-accent-ring',
};

const sizeClasses: Record<string, string> = {
  sm: 'h-8 px-2 text-xs',
  default: 'h-10 px-3 text-sm',
  lg: 'h-12 px-4 text-base',
};

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  (
    { className, type = 'text', variant = 'default', size = 'default', disabled, ...props },
    ref,
  ) => {
    const baseClasses =
      'flex w-full rounded-md border bg-background py-2 ' +
      'ring-offset-background ' +
      'file:border-0 file:bg-transparent file:text-sm file:font-medium ' +
      'placeholder:text-muted-foreground ' +
      'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 ' +
      'disabled:cursor-not-allowed disabled:opacity-50';

    const cls = classy(
      baseClasses,
      variantClasses[variant] ?? variantClasses.default,
      sizeClasses[size] ?? sizeClasses.default,
      className,
    );

    return (
      <input
        type={type}
        className={cls}
        ref={ref}
        disabled={disabled}
        aria-disabled={disabled ? 'true' : undefined}
        aria-invalid={variant === 'destructive' ? 'true' : undefined}
        {...props}
      />
    );
  },
);

Input.displayName = 'Input';

export default Input;
