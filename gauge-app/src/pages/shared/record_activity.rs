//! The Activity tab on a permission, group or domain page: When, Who and What changed, by
//! name, with a link to the Activity page filtered to the record.
//!
//! It replaced the Show History dialog. Gauge admins and the record's owners can read it;
//! anyone else sees the error bar's "You don't have access".

use gauge::types::{RecordKind, RecordRef};
use leptos::prelude::*;
use uf_product::components::{Body1, EmptyState};
use uf_product::primitives::{
    Flex, FlexGap, Link, SkeletonItem, Table, TableBody, TableCell, TableHeader, TableHeaderCell,
    TableRow,
};

use crate::components::error_bar::ErrorBar;
use crate::pages::shared::words::when_view;
use crate::server::get_record_activity;

/// The Activity page link for one record.
#[must_use]
pub fn activity_href(record: &RecordRef) -> String {
    let kind = match record.kind {
        RecordKind::Permission => "permission",
        RecordKind::Group => "group",
        RecordKind::Domain => "domain",
        RecordKind::Request => "request",
    };
    format!("/permission/activity?kind={kind}&record={}", record.id)
}

/// Noun for the "All activity for this …" link.
const fn noun(kind: RecordKind) -> &'static str {
    match kind {
        RecordKind::Permission => "permission",
        RecordKind::Group => "group",
        RecordKind::Domain => "domain",
        RecordKind::Request => "request",
    }
}

/// The Activity tab body for `record`. Loads when it mounts, in its own `Suspense`, so the
/// page around it never falls back.
#[component]
pub fn RecordActivity(
    /// The record whose changes to list.
    record: RecordRef,
) -> impl IntoView {
    let record = StoredValue::new(record);
    let refresh = RwSignal::new(0u32);
    let rows = Resource::new(
        move || refresh.get(),
        move |_| get_record_activity(record.get_value()),
    );
    let skeleton = || {
        view! {
            <Flex vertical=true gap=FlexGap::Small>
                {(0..3).map(|_| view! { <SkeletonItem width="100%".to_string() /> }).collect_view()}
            </Flex>
        }
    };
    view! {
        <div data-testid="gauge-record-activity">
            <Suspense fallback=skeleton>
                {move || rows.get().map(|result| match result {
                    Ok(items) if items.is_empty() => view! {
                        <EmptyState message="No changes yet." />
                    }.into_any(),
                    Ok(items) => view! {
                        <Flex vertical=true gap=FlexGap::Small>
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell>"When"</TableHeaderCell>
                                        <TableHeaderCell>"Who"</TableHeaderCell>
                                        <TableHeaderCell>"What changed"</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {items.into_iter().map(|row| view! {
                                        <TableRow attr:data-testid="gauge-record-activity-row">
                                            <TableCell>{when_view(row.at.to_rfc3339())}</TableCell>
                                            <TableCell><Body1>{row.actor_label}</Body1></TableCell>
                                            <TableCell><Body1>{row.summary}</Body1></TableCell>
                                        </TableRow>
                                    }).collect_view()}
                                </TableBody>
                            </Table>
                            <span data-testid="gauge-record-activity-all">
                                <Link href=activity_href(&record.get_value())>
                                    {format!("All activity for this {}", noun(record.get_value().kind))}
                                </Link>
                            </span>
                        </Flex>
                    }.into_any(),
                    Err(err) => view! {
                        <ErrorBar
                            error=err.to_string()
                            what="this activity"
                            on_retry=Callback::new(move |()| refresh.update(|n| *n += 1))
                        />
                    }.into_any(),
                })}
            </Suspense>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_link_filters_to_the_record_happy() {
        let record = RecordRef {
            kind: RecordKind::Group,
            id: "grp1".into(),
            label: "Release Captains".into(),
        };
        assert_eq!(
            activity_href(&record),
            "/permission/activity?kind=group&record=grp1"
        );
    }
}
