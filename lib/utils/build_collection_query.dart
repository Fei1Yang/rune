import 'package:player/messages/collection.pb.dart';
import 'package:player/utils/context_menu/collection_item_context_menu.dart';

List<(String, String)> buildCollectionQuery(
    CollectionType collectionType, int id) {
  if (collectionType == CollectionType.Mix) {
    throw "Not Allow";
  }
  return [(typeToOperator[collectionType]!, id.toString())];
}
