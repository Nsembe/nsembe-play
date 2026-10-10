import * as React from "react";
import { Switch as SwitchPrimitive, Tooltip as T } from "radix-ui";
import { cn } from "@/lib/utils";

export function Input({ className, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      className={cn(
        "h-10 w-full rounded-lg bg-white/[0.06] px-3 text-sm text-fg placeholder:text-subtle outline-none ring-1 ring-transparent transition focus:bg-white/[0.08] focus:ring-accent/50",
        className,
      )}
      {...props}
    />
  );
}

export function Textarea({ className, ...props }: React.ComponentProps<"textarea">) {
  return (
    <textarea
      className={cn(
        "min-h-20 w-full resize-none rounded-lg bg-white/[0.06] px-3 py-2.5 text-sm text-fg placeholder:text-subtle outline-none ring-1 ring-transparent transition focus:bg-white/[0.08] focus:ring-accent/50",
        className,
      )}
      {...props}
    />
  );
}

export function Switch({ className, ...props }: React.ComponentProps<typeof SwitchPrimitive.Root>) {
  return (
    <SwitchPrimitive.Root
      className={cn(
        "relative inline-flex h-6 w-11 shrink-0 items-center rounded-full bg-white/15 transition-colors data-[state=checked]:bg-accent",
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb className="block size-5 translate-x-0.5 rounded-full bg-white shadow transition-transform data-[state=checked]:translate-x-[22px] data-[state=checked]:bg-forest" />
    </SwitchPrimitive.Root>
  );
}

export function Skeleton({ className, ...props }: React.ComponentProps<"div">) {
  return <div className={cn("animate-pulse rounded-lg bg-white/[0.05]", className)} {...props} />;
}

export const TooltipProvider = T.Provider;

/** Small hover label. */
export function Tip({
  label,
  children,
  side = "top",
}: {
  label: React.ReactNode;
  children: React.ReactElement;
  side?: "top" | "bottom" | "left" | "right";
}) {
  return (
    <T.Root>
      <T.Trigger asChild>{children}</T.Trigger>
      <T.Portal>
        <T.Content
          side={side}
          sideOffset={6}
          className="z-[60] rounded-md bg-surface-3 px-2 py-1 text-xs font-medium text-fg shadow-pop data-[state=delayed-open]:animate-fade-in"
        >
          {label}
        </T.Content>
      </T.Portal>
    </T.Root>
  );
}
