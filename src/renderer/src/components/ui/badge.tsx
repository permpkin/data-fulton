import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../../lib/utils';

const badgeVariants = cva(
  'inline-flex items-center justify-center rounded-md border px-2 py-0.5 text-xs font-medium w-fit whitespace-nowrap shrink-0 gap-1 transition-[color,box-shadow] overflow-hidden',
  {
    variants: {
      variant: {
        default: 'border-transparent bg-primary text-primary-foreground',
        secondary: 'border-transparent bg-secondary text-secondary-foreground',
        destructive: 'border-transparent bg-destructive text-white dark:bg-destructive/60',
        outline: 'text-foreground',
        running: 'border-transparent bg-blue-500/15 text-blue-400',
        complete: 'border-transparent bg-emerald-500/15 text-emerald-400',
        paused: 'border-transparent bg-amber-500/15 text-amber-400',
        error: 'border-transparent bg-destructive/15 text-destructive',
      },
    },
    defaultVariants: { variant: 'default' },
  },
);

export function Badge({
  className,
  variant,
  ...props
}: React.ComponentProps<'span'> & VariantProps<typeof badgeVariants>) {
  return <span data-slot="badge" className={cn(badgeVariants({ variant }), className)} {...props} />;
}

export function statusVariant(status: string) {
  if (status === 'running') return 'running';
  if (status === 'complete' || status === 'done') return 'complete';
  if (status === 'paused' || status === 'pending') return 'paused';
  if (status === 'error' || status === 'cancelled') return 'error';
  return 'default';
}
