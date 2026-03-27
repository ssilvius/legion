/**
 * Multi-line text input component for longer form content
 *
 * @cognitive-load 4/10 - Extended input requires sustained attention for composition
 * @attention-economics Expands to accommodate content, focus state indicates active editing
 * @trust-building Auto-resize feedback, character count guidance, draft persistence patterns
 * @accessibility Screen reader labels, keyboard navigation, proper focus states
 * @semantic-meaning Extended text input: comments, descriptions, messages, notes
 *
 * @usage-patterns
 * DO: Always pair with descriptive Label component
 * DO: Provide placeholder text showing expected content format
 * DO: Use appropriate min/max heights for expected content length
 * DO: Consider character limits with visible counter
 * NEVER: Use for single-line input, use without associated label
 *
 * @example
 * ```tsx
 * <Label htmlFor="message">Message</Label>
 * <Textarea id="message" placeholder="Type your message here..." />
 * ```
 */
import * as React from 'react';
import classy from '@/src/lib/primitives/classy';

export interface TextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
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
  sm: 'min-h-16 px-2 py-1 text-xs',
  default: 'min-h-20 px-3 py-2 text-sm',
  lg: 'min-h-28 px-4 py-3 text-base',
};

export const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ className, disabled, variant = 'default', size = 'default', ...props }, ref) => {
    const baseClasses =
      'flex w-full rounded-md border bg-background ' +
      'ring-offset-background ' +
      'placeholder:text-muted-foreground ' +
      'focus-visible:outline-none focus-visible:ring-offset-2 ' +
      'disabled:cursor-not-allowed disabled:opacity-50';

    const cls = classy(
      baseClasses,
      variantClasses[variant] ?? variantClasses.default,
      sizeClasses[size] ?? sizeClasses.default,
      className,
    );

    return (
      <textarea
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

Textarea.displayName = 'Textarea';

export default Textarea;
