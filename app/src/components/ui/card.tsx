/**
 * Flexible container component for grouping related content with semantic structure
 *
 * @cognitive-load 2/10 - Simple container with clear boundaries and minimal cognitive overhead
 * @attention-economics Neutral container: Content drives attention, elevation hierarchy for interactive states
 * @trust-building Consistent spacing, predictable interaction patterns, clear content boundaries
 * @accessibility Proper heading structure, landmark roles, keyboard navigation for interactive cards
 * @semantic-meaning Structural roles: article=standalone content, section=grouped content, aside=supplementary information
 *
 * @usage-patterns
 * DO: Group related information with clear visual boundaries
 * DO: Create interactive cards with hover states and focus management
 * DO: Establish information hierarchy with header, content, actions
 * DO: Implement responsive scaling with consistent proportions
 * NEVER: Use decorative containers without semantic purpose
 * NEVER: Nest cards within cards
 * NEVER: Use Card for layout (use Grid/Container instead)
 *
 * @example
 * ```tsx
 * // Standalone content - use article
 * <Card as="article">
 *   <CardHeader>
 *     <CardTitle>Blog Post Title</CardTitle>
 *     <CardDescription>Published Jan 2025</CardDescription>
 *   </CardHeader>
 *   <CardContent>Post excerpt...</CardContent>
 * </Card>
 *
 * // Interactive card - product listing
 * <Card interactive>
 *   <CardHeader>
 *     <CardTitle>Product Name</CardTitle>
 *   </CardHeader>
 *   <CardContent>$99.00</CardContent>
 *   <CardFooter>
 *     <Button>Add to Cart</Button>
 *   </CardFooter>
 * </Card>
 *
 * // Supplementary content - use aside
 * <Card as="aside">
 *   <CardHeader>
 *     <CardTitle>Related Links</CardTitle>
 *   </CardHeader>
 *   <CardContent>...</CardContent>
 * </Card>
 * ```
 */
import * as React from 'react';
import { useCallback, useRef } from 'react';
import classy from '@/src/lib/primitives/classy';

// ============================================================================
// Card Context (R-202)
// ============================================================================

interface CardContextValue {
  editable: boolean | undefined;
  onTitleChange: ((title: string) => void) | undefined;
  onDescriptionChange: ((description: string) => void) | undefined;
}

const CardContext = React.createContext<CardContextValue | null>(null);

function useCardContext() {
  return React.useContext(CardContext);
}

// ============================================================================
// Card Props
// ============================================================================

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  as?: 'div' | 'article' | 'section' | 'aside';
  interactive?: boolean;
  /** Size variant for compact cards */
  size?: 'default' | 'sm';

  // ============================================================================
  // Editable Props (R-202)
  // ============================================================================

  /**
   * Enable editing mode for block editor
   * Makes CardTitle and CardDescription contenteditable
   */
  editable?: boolean | undefined;

  /**
   * Called when CardTitle text changes (if editable)
   */
  onTitleChange?: ((title: string) => void) | undefined;

  /**
   * Called when CardDescription text changes (if editable)
   */
  onDescriptionChange?: ((description: string) => void) | undefined;
}

export const Card = React.forwardRef<HTMLDivElement, CardProps>(
  (
    {
      as: Component = 'div',
      interactive,
      size = 'default',
      editable,
      onTitleChange,
      onDescriptionChange,
      className,
      children,
      ...props
    },
    ref,
  ) => {
    const base = 'bg-card text-card-foreground border border-card-border rounded-lg shadow-sm';

    // Size variants (shadcn v4 compatibility)
    const sizeStyles = size === 'sm' ? 'group/card-sm' : '';

    const interactiveStyles = interactive
      ? 'hover:bg-card-hover hover:shadow-md transition-shadow duration-normal motion-reduce:transition-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2'
      : '';

    // Editable mode styling (R-202)
    const editableStyles = editable
      ? 'outline-2 outline-dashed outline-muted-foreground/30 outline-offset-2'
      : '';

    const cls = classy(base, sizeStyles, interactiveStyles, editableStyles, className);

    const contextValue: CardContextValue = {
      editable,
      onTitleChange,
      onDescriptionChange,
    };

    return (
      <CardContext.Provider value={contextValue}>
        <Component
          ref={ref}
          className={cls}
          tabIndex={interactive ? 0 : undefined}
          data-editable={editable || undefined}
          {...props}
        >
          {children}
        </Component>
      </CardContext.Provider>
    );
  },
);

Card.displayName = 'Card';

export interface CardHeaderProps extends React.HTMLAttributes<HTMLDivElement> {}

export const CardHeader = React.forwardRef<HTMLDivElement, CardHeaderProps>(
  ({ className, ...props }, ref) => {
    const cls = classy('flex flex-col gap-1.5 p-6', className);
    return <div ref={ref} data-slot="card-header" className={cls} {...props} />;
  },
);

CardHeader.displayName = 'CardHeader';

export interface CardTitleProps extends React.HTMLAttributes<HTMLHeadingElement> {
  as?: 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6';
  /** Placeholder text shown when empty in edit mode */
  placeholder?: string | undefined;
}

export const CardTitle = React.forwardRef<HTMLHeadingElement, CardTitleProps>(
  ({ as: Component = 'h3', className, placeholder = 'Add title...', children, ...props }, ref) => {
    const context = useCardContext();
    const elementRef = useRef<HTMLHeadingElement>(null);

    const handleInput = useCallback(() => {
      if (!elementRef.current || !context?.onTitleChange) return;
      const text = elementRef.current.textContent ?? '';
      context.onTitleChange(text);
    }, [context]);

    // Prevent Enter from inserting line breaks in titles
    const handleKeyDown = useCallback((event: React.KeyboardEvent) => {
      if (event.key === 'Enter') {
        event.preventDefault();
      }
    }, []);

    // Strip formatting on paste - titles are plain text
    const handlePaste = useCallback((event: React.ClipboardEvent) => {
      event.preventDefault();
      const text = event.clipboardData.getData('text/plain').replace(/[\r\n]+/g, ' ');
      const selection = window.getSelection();
      if (selection?.rangeCount) {
        const range = selection.getRangeAt(0);
        range.deleteContents();
        range.insertNode(document.createTextNode(text));
        range.collapse(false);
      }
    }, []);

    // Combine refs
    const combinedRef = (element: HTMLHeadingElement | null) => {
      (elementRef as React.MutableRefObject<HTMLHeadingElement | null>).current = element;
      if (typeof ref === 'function') {
        ref(element);
      } else if (ref) {
        (ref as React.MutableRefObject<HTMLHeadingElement | null>).current = element;
      }
    };

    const cls = classy(
      'text-2xl font-semibold leading-none tracking-tight',
      context?.editable && 'outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 rounded',
      className,
    );

    const editableProps = context?.editable
      ? {
          contentEditable: true,
          suppressContentEditableWarning: true,
          onInput: handleInput,
          onKeyDown: handleKeyDown,
          onPaste: handlePaste,
          'data-placeholder': placeholder,
          'aria-placeholder': placeholder,
        }
      : {};

    return (
      <Component ref={combinedRef} className={cls} {...editableProps} {...props}>
        {children}
      </Component>
    );
  },
);

CardTitle.displayName = 'CardTitle';

export interface CardDescriptionProps extends React.HTMLAttributes<HTMLParagraphElement> {
  /** Placeholder text shown when empty in edit mode */
  placeholder?: string | undefined;
}

export const CardDescription = React.forwardRef<HTMLParagraphElement, CardDescriptionProps>(
  ({ className, placeholder = 'Add description...', children, ...props }, ref) => {
    const context = useCardContext();
    const elementRef = useRef<HTMLParagraphElement>(null);

    const handleInput = useCallback(() => {
      if (!elementRef.current || !context?.onDescriptionChange) return;
      const text = elementRef.current.textContent ?? '';
      context.onDescriptionChange(text);
    }, [context]);

    // Prevent Enter from inserting line breaks in descriptions
    const handleKeyDown = useCallback((event: React.KeyboardEvent) => {
      if (event.key === 'Enter') {
        event.preventDefault();
      }
    }, []);

    // Strip formatting on paste - descriptions are plain text
    const handlePaste = useCallback((event: React.ClipboardEvent) => {
      event.preventDefault();
      const text = event.clipboardData.getData('text/plain').replace(/[\r\n]+/g, ' ');
      const selection = window.getSelection();
      if (selection?.rangeCount) {
        const range = selection.getRangeAt(0);
        range.deleteContents();
        range.insertNode(document.createTextNode(text));
        range.collapse(false);
      }
    }, []);

    // Combine refs
    const combinedRef = (element: HTMLParagraphElement | null) => {
      (elementRef as React.MutableRefObject<HTMLParagraphElement | null>).current = element;
      if (typeof ref === 'function') {
        ref(element);
      } else if (ref) {
        (ref as React.MutableRefObject<HTMLParagraphElement | null>).current = element;
      }
    };

    const cls = classy(
      'text-sm text-muted-foreground',
      context?.editable && 'outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 rounded',
      className,
    );

    const editableProps = context?.editable
      ? {
          contentEditable: true,
          suppressContentEditableWarning: true,
          onInput: handleInput,
          onKeyDown: handleKeyDown,
          onPaste: handlePaste,
          'data-placeholder': placeholder,
          'aria-placeholder': placeholder,
        }
      : {};

    return (
      <p ref={combinedRef} className={cls} {...editableProps} {...props}>
        {children}
      </p>
    );
  },
);

CardDescription.displayName = 'CardDescription';

export interface CardActionProps extends React.HTMLAttributes<HTMLDivElement> {}

export const CardAction = React.forwardRef<HTMLDivElement, CardActionProps>(
  ({ className, ...props }, ref) => {
    const cls = classy('col-start-2 row-span-2 row-start-1 self-start justify-self-end', className);
    return <div ref={ref} data-slot="card-action" className={cls} {...props} />;
  },
);

CardAction.displayName = 'CardAction';

export interface CardContentProps extends React.HTMLAttributes<HTMLDivElement> {}

export const CardContent = React.forwardRef<HTMLDivElement, CardContentProps>(
  ({ className, ...props }, ref) => {
    const cls = classy('p-6 pt-0', className);
    return <div ref={ref} className={cls} {...props} />;
  },
);

CardContent.displayName = 'CardContent';

export interface CardFooterProps extends React.HTMLAttributes<HTMLDivElement> {}

export const CardFooter = React.forwardRef<HTMLDivElement, CardFooterProps>(
  ({ className, ...props }, ref) => {
    const cls = classy('flex items-center p-6 pt-0', className);
    return <div ref={ref} className={cls} {...props} />;
  },
);

CardFooter.displayName = 'CardFooter';

export default Card;
