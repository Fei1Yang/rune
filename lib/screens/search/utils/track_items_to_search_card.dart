import '../../../widgets/track_list/track_list.dart';
import '../../../screens/search/widgets/track_search_item.dart';

List<TrackSearchItem> trackItemsToSearchCard(List<InternalMediaFile> items) {
  final trackIds = items.map((x) => x.id).toList();

  return items
      .map(
        (a) => TrackSearchItem(
          index: 0,
          item: a,
          fallbackFileIds: trackIds,
        ),
      )
      .toList();
}
